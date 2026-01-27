#[cfg(test)]
mod named_struct_tests {
    use structure_of_arrays::{Aos, Soa, StructMetadata};
    use structure_of_arrays_macro::Aos;

    // We're testing this Aos derive
    #[derive(Copy, Clone, Default, Aos)]
    pub struct Sphere {
        pub position: [f32; 3],
        pub radius: f32,
        pub colour: [f32; 3],
        pub tag: u64,
    }

    #[test]
    fn layout_implemented() {
        assert!(Sphere::layout(0) == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::layout(1) == std::alloc::Layout::new::<f32>());
        assert!(Sphere::layout(2) == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::layout(3) == std::alloc::Layout::new::<u64>());
    }

    #[test]
    #[should_panic = "Too large index"]
    fn layout_panics_with_too_large_index() {
        // This should panic
        assert!(Sphere::layout(4) == std::alloc::Layout::new::<f32>());
    }

    #[test]
    fn offset_of_implemented() {
        assert!(Sphere::offset_of(0) == std::mem::offset_of!(Sphere, position));
        assert!(Sphere::offset_of(1) == std::mem::offset_of!(Sphere, radius));
        assert!(Sphere::offset_of(2) == std::mem::offset_of!(Sphere, colour));
        assert!(Sphere::offset_of(3) == std::mem::offset_of!(Sphere, tag));
    }

    #[test]
    #[should_panic = "Too large index"]
    fn offset_of_panics_with_too_large_index() {
        // This should panic
        assert!(Sphere::offset_of(4) == 0);
    }

    #[test]
    fn named_accessors_implemented() {
        const N: usize = 1 << 6;
        let mut soa = Soa::<Sphere, { Sphere::NUM_FIELDS }>::new(N);

        // Access either through the interface...
        let _ = SphereAccessor::position_mut(&mut soa);
        let _ = SphereAccessor::position(&soa);

        // ... or through the variable
        let _ = soa.radius_mut();
        let _ = soa.radius();

        let _ = soa.colour_mut();
        let _ = soa.colour();

        let _ = soa.tag_mut();
        let _ = soa.tag();
    }
}

#[cfg(test)]
mod unnamed_struct_tests {
    use structure_of_arrays::{Aos, Soa, StructMetadata};
    use structure_of_arrays_macro::Aos;

    // We're testing this Aos derive
    #[derive(Copy, Clone, Default, Aos)]
    pub struct Sphere(pub [f32; 3], pub f32, pub [f32; 3], pub u64);

    #[test]
    fn layout_implemented() {
        assert!(Sphere::layout(0) == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::layout(1) == std::alloc::Layout::new::<f32>());
        assert!(Sphere::layout(2) == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::layout(3) == std::alloc::Layout::new::<u64>());
    }

    #[test]
    #[should_panic = "Too large index"]
    fn layout_panics_with_too_large_index() {
        // This should panic
        assert!(Sphere::layout(4) == std::alloc::Layout::new::<f32>());
    }

    #[test]
    fn offset_of_implemented() {
        assert!(Sphere::offset_of(0) == std::mem::offset_of!(Sphere, 0));
        assert!(Sphere::offset_of(1) == std::mem::offset_of!(Sphere, 1));
        assert!(Sphere::offset_of(2) == std::mem::offset_of!(Sphere, 2));
        assert!(Sphere::offset_of(3) == std::mem::offset_of!(Sphere, 3));
    }

    #[test]
    #[should_panic = "Too large index"]
    fn offset_of_panics_with_too_large_index() {
        // This should panic
        assert!(Sphere::offset_of(4) == 0);
    }

    #[test]
    fn named_accessors_implemented() {
        const N: usize = 1 << 6;
        let mut soa = Soa::<Sphere, { Sphere::NUM_FIELDS }>::new(N);

        // Access either through the interface...
        let _ = SphereAccessor::field0_mut(&mut soa);
        let _ = SphereAccessor::field0(&soa);

        // ... or through the variable
        let _ = soa.field1_mut();
        let _ = soa.field1();

        let _ = soa.field2_mut();
        let _ = soa.field2();

        let _ = soa.field3_mut();
        let _ = soa.field3();
    }
}
