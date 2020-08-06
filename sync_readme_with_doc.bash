#!/bin/bash

{
    sed 's+^+//! +;s/ \+$//' README.md
    grep -v '^//!' src/lib.rs
} >src/lib.rs.tmp

mv src/lib.rs.tmp src/lib.rs
