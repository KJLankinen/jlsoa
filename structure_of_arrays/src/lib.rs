use std::{
    alloc,
    mem::{self, swap},
    ptr, slice,
};

pub trait StructMetadata<const N: usize> {
    const LAYOUT: [alloc::Layout; N];
    const OFFSET: [usize; N];
}

// TODO: think about thread safety and whether or not this should be split to an owning struct and
// views, which have a (possibly partial) view into the memory, which they may modify at will
#[derive(Debug)]
pub struct Soa<T, const N: usize> {
    pointers: [*mut u8; N],
    length: usize,
    capacity: usize,
    #[allow(dead_code)]
    data: Vec<u8>,
    marker: std::marker::PhantomData<T>,
}

struct StaticAssert<const N: usize, const I: usize> {}
impl<const N: usize, const I: usize> StaticAssert<N, I> {
    pub const GREATER_THAN: () = assert!(N > I);
}

impl<T, const N: usize> Soa<T, N>
where
    T: Copy + Default + StructMetadata<N>,
{
    pub fn new(capacity: usize) -> Self {
        let mut vec: Vec<u8> = vec![0u8; Self::memory_requirement(capacity)];
        let data_slice = vec.as_mut_slice();
        let mut ptr: *mut u8 = data_slice.as_mut_ptr();

        let soa = Self {
            pointers: std::array::from_fn(|i| {
                let layout = T::LAYOUT[i];
                let alignment = ptr.align_offset(layout.align());

                // size is less than usize::MAX
                assert!(usize::MAX / layout.size() > capacity);
                let size = capacity * layout.size();

                // size + alignment are less than isize::MAX
                assert!(alignment + size < isize::MAX as usize);

                // Safety:
                // - if i == 0, this is safe, since these are the same ptr
                // - if i > 0, this is safe because below we assert
                //   that the new pointer stays within the allocated object
                let offset = unsafe { ptr.byte_offset_from(data_slice.as_ptr()) };
                let new_offset = (alignment + size) as isize;

                // offset + new_offset doesn't overflow isize
                assert!(offset < isize::MAX - new_offset);
                // offset + new_offset is within the allocated object
                assert!(offset + new_offset <= data_slice.len() as isize);

                unsafe {
                    // This is safe, because ptr + alignment + size stays within
                    // the allocated object as per the assertions above
                    let aligned = ptr.byte_offset(alignment as isize);
                    ptr = aligned.byte_offset(size as isize);
                    aligned
                }
            }),
            length: 0,
            capacity,
            data: vec,
            marker: std::marker::PhantomData,
        };

        // N.B. !! The memory is just raw bytes, the pattern of which can be arbitrary: we need to
        // set the values to the defaults for each type.
        let default = &T::default();
        let byte_ptr = (default as *const T).cast::<u8>();
        for i in 0..N {
            let size_bytes = T::LAYOUT[i].size();
            let offset_bytes = T::OFFSET[i];

            assert!(offset_bytes + size_bytes <= mem::size_of::<T>());

            unsafe {
                // Safe because offset is within the object
                let src = byte_ptr.add(offset_bytes);

                for j in 0..capacity {
                    // Safe, because we're doing this for values from 0 to capacity, where capacity is the amount
                    // of elements
                    let dst = soa.pointers[i].add(size_bytes * j);
                    // Safe, since we're copying from a distinct object to our memory
                    ptr::copy_nonoverlapping(src, dst, size_bytes);
                }
            }
        }
        soa
    }

    pub fn memory_requirement(capacity: usize) -> usize {
        (0..N)
            .map(|i| {
                let layout = T::LAYOUT[i];
                capacity * layout.size() + layout.align() - 1
            })
            .sum()
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn swap(&mut self, other: &mut Self) {
        swap(self, other);
    }

    pub fn push(&mut self, value: &T) {
        if self.length >= self.capacity {
            // Reallocate & copy
            let mut soa = Self::new(2 * self.capacity);
            for i in 0..N {
                let size_bytes = T::LAYOUT[i].size();
                unsafe {
                    let src = self.pointers[i];
                    let dst = soa.pointers[i];

                    // Safe, since soa is different from self, and thus their pointers are distinct
                    // Thus, the &T cannot point to our memory
                    ptr::copy_nonoverlapping(src, dst, size_bytes * self.length);
                }
            }
            soa.length = self.length;

            self.swap(&mut soa);
        }

        let byte_ptr = (value as *const T).cast::<u8>();
        for i in 0..N {
            let size_bytes = T::LAYOUT[i].size();
            let offset_bytes = T::OFFSET[i];

            assert!(offset_bytes + size_bytes <= mem::size_of::<T>());

            unsafe {
                // Safe because offset is within the object
                let src = byte_ptr.add(offset_bytes);
                // Safe because we checked we're not at full capacity
                let dst = self.pointers[i].add(size_bytes * self.length);
                // Safe, since our pointers store the individual members of T, not T itself.
                // Thus, the &T cannot point to our memory
                ptr::copy_nonoverlapping(src, dst, size_bytes);
            }
        }

        self.length += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let value = self.get_copy(self.length - 1);
            self.length -= 1;
            Some(value)
        }
    }

    pub fn swap_remove(&mut self, index: usize) -> T {
        assert!(index < self.len());
        self.swap_remove_unchecked(index)
    }

    pub fn swap_remove_unchecked(&mut self, index: usize) -> T {
        if index == self.len() - 1 {
            self.pop().unwrap()
        } else {
            // last != index as per the check above
            let last = self.len() - 1;
            let mut uninit: mem::MaybeUninit<T> = mem::MaybeUninit::uninit();
            let byte_ptr: *mut u8 = uninit.as_mut_ptr().cast::<u8>();

            for i in 0..N {
                let size_bytes = T::LAYOUT[i].size();
                let offset_bytes = T::OFFSET[i];

                assert!(offset_bytes + size_bytes <= mem::size_of::<T>());
                unsafe {
                    // 1. Copy from index to returned value
                    let src = self.pointers[i].add(size_bytes * index);
                    // Safe because offset is within the object
                    let dst = byte_ptr.add(offset_bytes);
                    // Safe because we're copying from our pointers array to a struct on the stack
                    ptr::copy_nonoverlapping(src, dst, size_bytes);

                    // 2. Copy from last to index
                    let dst = src;
                    // Safe because last is within bounds
                    let src = self.pointers[i].add(size_bytes * last);
                    // Safe because last != index as per the checks above
                    ptr::copy_nonoverlapping(src, dst, size_bytes);
                }
            }

            unsafe { uninit.assume_init() }
        }
    }

    pub fn get_copy(&self, index: usize) -> T {
        assert!(index < self.len());
        self.get_copy_unchecked(index)
    }

    pub fn get_copy_unchecked(&self, index: usize) -> T {
        let mut uninit: mem::MaybeUninit<T> = mem::MaybeUninit::uninit();
        let byte_ptr: *mut u8 = uninit.as_mut_ptr().cast::<u8>();

        for i in 0..N {
            let size_bytes = T::LAYOUT[i].size();
            let offset_bytes = T::OFFSET[i];

            assert!(offset_bytes + size_bytes <= mem::size_of::<T>());
            unsafe {
                // Safe because we checked index is below self.length
                let src = self.pointers[i].add(size_bytes * index);
                // Safe because offset is within the object
                let dst = byte_ptr.add(offset_bytes);
                // Safe because we're copying from our pointers array to a struct on the stack
                ptr::copy_nonoverlapping(src, dst, size_bytes);
            }
        }

        unsafe { uninit.assume_init() }
    }

    pub fn get_copies(&self) -> Vec<T> {
        (0..self.length)
            .map(|i| self.get_copy(i))
            .collect::<Vec<T>>()
    }

    pub fn get_slice<S, const I: usize>(&self) -> &[S] {
        let _ = StaticAssert::<N, I>::GREATER_THAN;
        unsafe { slice::from_raw_parts(self.pointers[I].cast::<S>(), self.len()) }
    }

    pub fn get_mut_slice<S, const I: usize>(&mut self) -> &mut [S] {
        let _ = StaticAssert::<N, I>::GREATER_THAN;
        unsafe { slice::from_raw_parts_mut(self.pointers[I].cast::<S>(), self.len()) }
    }
}

#[cfg(test)]
mod tests {
    use std::mem;
    use std::slice;

    use super::{Soa, StructMetadata};
    use structure_of_arrays_macro::Aos;

    #[allow(dead_code)]
    #[derive(Debug, Clone, Copy, Default, Aos)]
    struct Sphere {
        radius: f32,
        position: [f32; 3],
        tag: u64,
    }

    type SphereSoa = super::Soa<Sphere, { Sphere::LAYOUT.len() }>;

    #[test]
    fn mem_req1() {
        let mem_req = SphereSoa::memory_requirement(1);
        assert!(mem_req > std::mem::size_of::<Sphere>());
    }

    #[test]
    fn mem_req2() {
        const N: usize = 256;
        let mem_req = SphereSoa::memory_requirement(N);
        assert!(mem_req > N * std::mem::size_of::<Sphere>());
    }

    #[test]
    fn mem_req3() {
        const N: usize = 1 << 20;
        let mem_req = SphereSoa::memory_requirement(N);
        assert!(mem_req > N * std::mem::size_of::<Sphere>());
        println!("{}, {}", mem_req, N * std::mem::size_of::<Sphere>());
    }

    #[test]
    fn new() {
        const N: usize = 1 << 20;
        let soa = SphereSoa::new(N);
        let data_end = unsafe { soa.data.as_ptr().add(soa.data.len()) };

        assert!(soa.pointers[0].cast::<f32>().is_aligned());
        assert!(soa.pointers[1].cast::<[f32; 3]>().is_aligned());
        assert!(soa.pointers[2].cast::<u64>().is_aligned());

        unsafe {
            assert!(
                soa.pointers[1].offset_from(soa.pointers[0])
                    >= (N * std::mem::size_of::<f32>()) as isize
            );
            let end = soa.pointers[0].cast::<f32>().add(N).cast::<u8>();
            assert!(soa.pointers[1].offset_from(end) >= 0);

            assert!(
                soa.pointers[2].offset_from(soa.pointers[1])
                    >= (N * std::mem::size_of::<[f32; 3]>()) as isize
            );
            let end = soa.pointers[1].cast::<[f32; 3]>().add(N).cast::<u8>();
            assert!(soa.pointers[2].offset_from(end) >= 0);

            assert!(
                data_end.offset_from(soa.pointers[2]) >= (N * std::mem::size_of::<u64>()) as isize
            );
            let end = soa.pointers[2].cast::<u64>().add(N).cast::<u8>();
            assert!(data_end.offset_from(end) >= 0);
        }

        assert!(soa.is_empty());
    }

    #[test]
    fn swapping_two_soas_works() {
        let mut soa1 = SphereSoa::new(8);

        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa1.push(&sphere);
        soa1.push(&sphere);
        soa1.push(&sphere);
        soa1.push(&sphere);

        assert_eq!(soa1.capacity, 8);
        assert_eq!(soa1.length, 4);

        let spheres = soa1.get_copies();
        for sphere in spheres {
            assert_eq!(sphere.radius, 1.0);
            assert_eq!(sphere.position, [1.0, 2.0, 3.0]);
            assert_eq!(sphere.tag, 666);
        }

        let mut soa2 = SphereSoa::new(16);
        soa1.swap(&mut soa2);

        assert_eq!(soa2.capacity, 8);
        assert_eq!(soa2.length, 4);

        let spheres = soa2.get_copies();
        for sphere in spheres {
            assert_eq!(sphere.radius, 1.0);
            assert_eq!(sphere.position, [1.0, 2.0, 3.0]);
            assert_eq!(sphere.tag, 666);
        }

        assert!(soa1.is_empty());
        assert_eq!(soa1.capacity, 16);
    }

    #[test]
    fn lenght_increased_correctly_when_pushed() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);

        assert!(soa.is_empty());
        soa.push(&Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        });
        assert!(soa.len() == 1);
    }

    #[test]
    fn pop_returns_pushed() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);
        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa.push(&sphere);
        let sphere2 = soa.pop().unwrap();

        assert!(sphere.radius == sphere2.radius);
        assert!(sphere.position == sphere2.position);
        assert!(sphere.tag == sphere2.tag);

        assert!(soa.is_empty());
    }

    #[test]
    fn pushed_is_set_correctly() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);
        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa.push(&sphere);

        assert!(sphere.radius == soa.get_slice::<f32, 0>()[0]);
        assert!(sphere.position == soa.get_slice::<[f32; 3], 1>()[0]);
        assert!(sphere.tag == soa.get_slice::<u64, 2>()[0]);
    }

    #[test]
    fn push_reallocated_at_capacity() {
        let mut soa = SphereSoa::new(8);
        assert_eq!(soa.capacity, 8);

        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        for _ in 0..8 {
            soa.push(&sphere);
        }

        soa.push(&sphere);
        assert!(soa.capacity > 8);
        assert_eq!(soa.length, 9);

        for sphere in soa.get_copies() {
            assert_eq!(sphere.radius, 1.0);
            assert_eq!(sphere.position, [1.0, 2.0, 3.0]);
            assert_eq!(sphere.tag, 666);
        }
    }

    #[test]
    fn slice_mut_mutates() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);
        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa.push(&sphere);

        assert!(sphere.radius == soa.get_slice::<f32, 0>()[0]);
        soa.get_mut_slice::<f32, 0>()[0] = 666.666;
        assert!(666.666 == soa.get_slice::<f32, 0>()[0]);
    }

    #[test]
    fn get_copy_copies_correctly() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);
        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa.push(&sphere);
        let sphere2 = soa.get_copy(0);

        assert!(sphere.radius == sphere2.radius);
        assert!(sphere.position == sphere2.position);
        assert!(sphere.tag == sphere2.tag);

        assert!(soa.len() == 1);
    }

    #[test]
    #[should_panic = "assertion failed: index < self.len()"]
    fn get_copy_empty_returns_none() {
        const N: usize = 1 << 20;
        let soa = SphereSoa::new(N);
        // Panics because soa is empty
        let _ = soa.get_copy(0);
    }

    #[test]
    fn pop_empty_returns_none() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);
        assert!(soa.pop().is_none());
    }

    #[test]
    fn get_copies_copies_all_correctly() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);
        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa.push(&sphere);
        soa.push(&sphere);
        soa.push(&sphere);
        let spheres = soa.get_copies();

        assert!(soa.len() == 3);
        assert!(spheres.len() == 3);

        for sphere2 in spheres {
            assert!(sphere.radius == sphere2.radius);
            assert!(sphere.position == sphere2.position);
            assert!(sphere.tag == sphere2.tag);
        }
    }

    #[test]
    #[should_panic = "assertion failed: index < self.len()"]
    fn swap_remove_empty_panics() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);
        // Panic
        let _ = soa.swap_remove(0);
    }

    #[test]
    #[should_panic = "assertion failed: index < self.len()"]
    fn swap_remove_too_large_index_panics() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);

        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa.push(&sphere);

        // Panic
        let _ = soa.swap_remove(1);
    }

    #[test]
    fn swap_remove_with_only_one_element_returns_the_only_and_soa_is_then_empty() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);

        let sphere = Sphere {
            radius: 1.0,
            position: [1.0, 2.0, 3.0],
            tag: 666,
        };

        soa.push(&sphere);

        let sphere2 = soa.swap_remove(0);

        assert!(sphere.radius == sphere2.radius);
        assert!(sphere.position == sphere2.position);
        assert!(sphere.tag == sphere2.tag);

        assert!(soa.is_empty());
    }

    #[test]
    fn swap_remove_removes_the_correct_element1() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);

        let sphere = Sphere {
            tag: 0,
            ..Default::default()
        };
        let sphere2 = Sphere {
            tag: 1,
            ..Default::default()
        };
        let sphere3 = Sphere {
            tag: 2,
            ..Default::default()
        };

        soa.push(&sphere);
        soa.push(&sphere2);
        soa.push(&sphere3);

        let sphere = soa.swap_remove(0);
        assert!(sphere.tag == 0);
        assert!(soa.tag()[0] == 2);
        assert!(soa.tag()[1] == 1);
    }

    #[test]
    fn swap_remove_removes_the_correct_element2() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);

        let sphere = Sphere {
            tag: 0,
            ..Default::default()
        };
        let sphere2 = Sphere {
            tag: 1,
            ..Default::default()
        };
        let sphere3 = Sphere {
            tag: 2,
            ..Default::default()
        };

        soa.push(&sphere);
        soa.push(&sphere2);
        soa.push(&sphere3);

        let sphere = soa.swap_remove(1);
        assert!(sphere.tag == 1);
        assert!(soa.tag()[0] == 0);
        assert!(soa.tag()[1] == 2);
    }

    #[test]
    fn swap_remove_removes_the_correct_element3() {
        const N: usize = 1 << 20;
        let mut soa = SphereSoa::new(N);

        let sphere = Sphere {
            tag: 0,
            ..Default::default()
        };
        let sphere2 = Sphere {
            tag: 1,
            ..Default::default()
        };
        let sphere3 = Sphere {
            tag: 2,
            ..Default::default()
        };

        soa.push(&sphere);
        soa.push(&sphere2);
        soa.push(&sphere3);

        let sphere = soa.swap_remove(2);
        assert!(sphere.tag == 2);
        assert!(soa.tag()[0] == 0);
        assert!(soa.tag()[1] == 1);
    }

    #[test]
    fn memory_initialized_correctly() {
        // Make things for which the default byte pattern is not 0
        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy, Default)]
        enum Foo {
            A,
            B,
            #[default]
            C,
        }

        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy)]
        struct Tag {
            value: f32,
        }

        impl Default for Tag {
            fn default() -> Self {
                Tag { value: 666.6 }
            }
        }

        #[derive(Debug, Clone, Copy, Default, Aos)]
        struct Bar {
            foo: Foo,
            tag: Tag,
        }

        type BarSoa = super::Soa<Bar, { Bar::LAYOUT.len() }>;

        let default = &Bar::default();
        let default_ptr = (default as *const Bar).cast::<u8>();

        const N: usize = 1 << 5;

        for i in 0..Bar::LAYOUT.len() {
            let size_bytes = Bar::LAYOUT[i].size();
            let offset_bytes = Bar::OFFSET[i];
            assert!(offset_bytes + size_bytes <= mem::size_of::<Bar>());
            // The byte pattern for the defaults should differ from zero for at least some bytes
            unsafe {
                let default_bytes =
                    slice::from_raw_parts(default_ptr.add(offset_bytes), size_bytes);
                let mut all_zero = true;
                for byte in default_bytes {
                    all_zero &= byte == &0u8;
                }

                assert!(!all_zero);
            }
        }

        let soa = BarSoa::new(N);
        for i in 0..Bar::LAYOUT.len() {
            let size_bytes = Bar::LAYOUT[i].size();
            let offset_bytes = Bar::OFFSET[i];

            assert!(offset_bytes + size_bytes <= mem::size_of::<Bar>());

            unsafe {
                // Safe because offset is within the object
                let default_bytes =
                    slice::from_raw_parts(default_ptr.add(offset_bytes), size_bytes);

                for j in 0..N {
                    // Safe, because we're doing this for values from 0 to N, where N is the amount
                    // of elements
                    let init_bytes =
                        slice::from_raw_parts(soa.pointers[i].add(size_bytes * j), size_bytes);

                    // Compare the bytes of the default object to the bytes of the soa:
                    // they should be equal
                    for k in 0..size_bytes {
                        assert_eq!(default_bytes[k], init_bytes[k]);
                    }
                }
            }
        }
    }
}
