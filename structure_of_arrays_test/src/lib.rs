#[cfg(test)]
mod named_struct_tests {
    use structure_of_arrays::{Soa, StructMetadata};
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
        assert!(Sphere::LAYOUT[0] == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::LAYOUT[1] == std::alloc::Layout::new::<f32>());
        assert!(Sphere::LAYOUT[2] == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::LAYOUT[3] == std::alloc::Layout::new::<u64>());
    }

    #[test]
    fn offset_of_implemented() {
        assert!(Sphere::OFFSET[0] == std::mem::offset_of!(Sphere, position));
        assert!(Sphere::OFFSET[1] == std::mem::offset_of!(Sphere, radius));
        assert!(Sphere::OFFSET[2] == std::mem::offset_of!(Sphere, colour));
        assert!(Sphere::OFFSET[3] == std::mem::offset_of!(Sphere, tag));
    }

    #[test]
    fn named_accessors_implemented() {
        const N: usize = 1 << 6;
        let mut soa = Soa::<Sphere, { Sphere::LAYOUT.len() }>::new(N);

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
    use structure_of_arrays::{Soa, StructMetadata};
    use structure_of_arrays_macro::Aos;

    // We're testing this Aos derive
    #[derive(Copy, Clone, Default, Aos)]
    pub struct Sphere(pub [f32; 3], pub f32, pub [f32; 3], pub u64);

    #[test]
    fn layout_implemented() {
        assert!(Sphere::LAYOUT[0] == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::LAYOUT[1] == std::alloc::Layout::new::<f32>());
        assert!(Sphere::LAYOUT[2] == std::alloc::Layout::new::<[f32; 3]>());
        assert!(Sphere::LAYOUT[3] == std::alloc::Layout::new::<u64>());
    }

    #[test]
    fn offset_of_implemented() {
        assert!(Sphere::OFFSET[0] == std::mem::offset_of!(Sphere, 0));
        assert!(Sphere::OFFSET[1] == std::mem::offset_of!(Sphere, 1));
        assert!(Sphere::OFFSET[2] == std::mem::offset_of!(Sphere, 2));
        assert!(Sphere::OFFSET[3] == std::mem::offset_of!(Sphere, 3));
    }

    #[test]
    fn named_accessors_implemented() {
        const N: usize = 1 << 6;
        let mut soa = Soa::<Sphere, { Sphere::LAYOUT.len() }>::new(N);

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
