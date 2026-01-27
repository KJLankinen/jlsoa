This directory contains a re-rexporter crate and three child crates:
- `structure_of_arrays`, which contains the generic code for using the structure of arrays
- `structure_of_arrays_macro`, which contains procedural macros used to generate helper code for the structure of arrays code
- `structure_of_arrays_test`, which contains code for testing the macro

The re-rexporter crate just exports things from `structure_of_arrays` and `structure_of_arrays_macro` so they can be used from a single crate where they're used.
