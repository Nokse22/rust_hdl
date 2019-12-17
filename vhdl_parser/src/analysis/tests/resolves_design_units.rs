// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2019, Olof Kraigher olof.kraigher@gmail.com

use super::*;

#[test]
fn error_on_configuration_before_entity_in_same_file() {
    let mut builder = LibraryBuilder::new();

    let code = builder.code(
        "libname",
        "
configuration cfg of ent is
for rtl
end for;
end configuration;

entity ent is
end entity;
",
    );

    check_diagnostics(
        builder.analyze(),
        vec![Diagnostic::error(
            code.s("cfg", 1),
            "Configuration 'cfg' declared before entity 'ent'",
        )],
    );
}

#[test]
fn error_on_configuration_of_missing_entity() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
configuration cfg of ent is
for rtl
end for;
end configuration;
",
    );

    check_diagnostics(
        builder.analyze(),
        vec![Diagnostic::error(
            code.s("ent", 1),
            "No declaration of 'ent'",
        )],
    );
}

#[test]
fn good_configurations() {
    let mut builder = LibraryBuilder::new();
    builder.code(
        "libname",
        "
entity ent is
end entity;

configuration cfg_good1 of ent is
for rtl
end for;
end configuration;

configuration cfg_good2 of work.ent is
for rtl
end for;
end configuration;

library libname;
configuration cfg_good3 of libname.ent is
for rtl
end for;
end configuration;
",
    );

    check_no_diagnostics(&builder.analyze());
}

#[test]
fn error_on_configuration_of_entity_outside_of_library() {
    let mut builder = LibraryBuilder::new();
    builder.code(
        "lib2",
        "
entity ent is
end entity;",
    );
    let code = builder.code(
        "libname",
        "
library lib2;

configuration cfg of lib2.ent is
for rtl
end for;
end configuration;
",
    );

    check_diagnostics(
        builder.analyze(),
        vec![Diagnostic::error(
            code.s("lib2.ent", 1),
            "Configuration must be within the same library 'libname' as the corresponding entity",
        )],
    );
}

#[test]
fn search_reference_from_configuration_to_entity() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ename is
end entity;

configuration cfg_good1 of ename is
for rtl
end for;
end configuration;
",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_no_diagnostics(&diagnostics);

    // From reference
    assert_eq!(
        root.search_reference(code.source(), code.s("ename", 2).start()),
        Some(code.s("ename", 1).pos())
    );

    // From declaration position
    assert_eq!(
        root.search_reference(code.source(), code.s("ename", 1).start()),
        Some(code.s("ename", 1).pos())
    );

    // Find all references
    assert_eq_unordered(
        &root.find_all_references(&code.s1("ename").pos()),
        &vec![code.s("ename", 1).pos(), code.s("ename", 2).pos()],
    );
}

#[test]
fn resolves_reference_from_architecture_to_entity() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ename1 is
end entity;

architecture a of ename1 is
begin
end architecture;
",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_no_diagnostics(&diagnostics);

    // From reference position
    assert_eq!(
        root.search_reference(code.source(), code.s("ename1", 2).start()),
        Some(code.s("ename1", 1).pos())
    );

    // Find all references
    assert_eq_unordered(
        &root.find_all_references(&code.s1("ename1").pos()),
        &vec![code.s("ename1", 1).pos(), code.s("ename1", 2).pos()],
    );
}

#[test]
fn resolves_reference_to_entity_instance() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ename1 is
end entity;

architecture a of ename1 is
begin
end architecture;

entity ename2 is
end entity;

architecture a of ename2 is
begin
  bad_inst : entity work.missing;
  inst : entity work.ename1;
end architecture;
",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_diagnostics(
        diagnostics,
        vec![Diagnostic::error(
            code.s1("missing"),
            "No primary unit 'missing' within 'libname'",
        )],
    );

    // From reference position
    assert_eq!(
        root.search_reference(code.source(), code.s("ename1", 3).start()),
        Some(code.s("ename1", 1).pos())
    );

    // Find all references
    assert_eq_unordered(
        &root.find_all_references(&code.s1("ename1").pos()),
        &vec![
            code.s("ename1", 1).pos(),
            code.s("ename1", 2).pos(),
            code.s("ename1", 3).pos(),
        ],
    );
}

#[test]
fn resolves_component_instance() {
    check_missing(
        "
entity ent is
end entity;

architecture a of ent is
begin
  inst : component missing;
end architecture;
",
    );
}

#[test]
fn search_component_instance() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
  component decl is
  end component;
begin
  inst : component decl;
end architecture;
",
    );
}

#[test]
fn resolves_configuration_instance() {
    check_missing(
        "
entity ent is
end entity;

architecture a of ent is
begin
  inst : configuration missing;
end architecture;
",
    );
}

#[test]
fn search_configuration_instance() {
    check_search_reference(
        "
entity ent is
end entity;

configuration decl of ent is
  for a
  end for;
end configuration;

architecture a of ent is
begin
  inst : configuration work.decl;
end architecture;


",
    );
}

#[test]
fn resolves_reference_to_package_body() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is
end package;

package body pkg is
end package body;
",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_no_diagnostics(&diagnostics);

    // From declaration position
    assert_eq!(
        root.search_reference(code.source(), code.s("pkg", 1).start()),
        Some(code.s("pkg", 1).pos())
    );

    // From reference position
    assert_eq!(
        root.search_reference(code.source(), code.s("pkg", 2).start()),
        Some(code.s("pkg", 1).pos())
    );

    // Find all references
    assert_eq_unordered(
        &root.find_all_references(&code.s1("pkg").pos()),
        &vec![code.s("pkg", 1).pos(), code.s("pkg", 2).pos()],
    );
}