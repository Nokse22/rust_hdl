// This Source Code Form is subject to the terms of the Mozilla Public
// Lic// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// This Source Code Form is subject to the terms of the Mozilla Public
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2019, Olof Kraigher olof.kraigher@gmail.com

use super::*;

#[must_use]
pub enum SearchResult<T> {
    Found(T),
    NotFound,
}

impl<T> Into<Option<T>> for SearchResult<T> {
    fn into(self) -> Option<T> {
        match self {
            Found(value) => Some(value),
            NotFound => None,
        }
    }
}

#[must_use]
pub enum SearchState<T> {
    Finished(SearchResult<T>),
    NotFinished,
}

pub use SearchResult::*;
pub use SearchState::*;

impl<T> SearchState<T> {
    fn or_else(self, nested_fun: impl FnOnce() -> SearchResult<T>) -> SearchResult<T> {
        match self {
            Finished(result) => result,
            NotFinished => nested_fun(),
        }
    }

    fn or_not_found(self) -> SearchResult<T> {
        self.or_else(|| NotFound)
    }
}

pub trait Searcher<T> {
    fn search_labeled_concurrent_statement(
        &mut self,
        _stmt: &LabeledConcurrentStatement,
    ) -> SearchState<T> {
        NotFinished
    }
    fn search_labeled_sequential_statement(
        &mut self,
        _stmt: &LabeledSequentialStatement,
    ) -> SearchState<T> {
        NotFinished
    }
    fn search_declaration(&mut self, _decl: &Declaration) -> SearchState<T> {
        NotFinished
    }
    fn search_interface_declaration(&mut self, _decl: &InterfaceDeclaration) -> SearchState<T> {
        NotFinished
    }
    fn search_subtype_indication(&mut self, _decl: &SubtypeIndication) -> SearchState<T> {
        NotFinished
    }
    fn search_designator_ref(&mut self, _designator: &WithRef<Designator>) -> SearchState<T> {
        NotFinished
    }
    fn search_with_pos(&mut self, _pos: &SrcPos) -> SearchState<T> {
        NotFinished
    }
    fn search_source(&mut self, _source: &Source) -> SearchState<T> {
        NotFinished
    }
}

pub trait Search<T> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T>;
}

#[macro_export]
macro_rules! return_if {
    ($result:expr) => {
        match $result {
            result @ Found(_) => {
                return result;
            }
            _ => {}
        };
    };
}

impl<T, V: Search<T>> Search<T> for Vec<V> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        for decl in self.iter() {
            return_if!(decl.search(searcher));
        }
        NotFound
    }
}

impl<T, V: Search<T>> Search<T> for Option<V> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        for decl in self.iter() {
            return_if!(decl.search(searcher));
        }
        NotFound
    }
}

impl<T> Search<T> for LabeledSequentialStatement {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_labeled_sequential_statement(self)
            .or_else(|| NotFound)
    }
}

impl<T> Search<T> for GenerateBody {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        if let Some(ref decl) = self.decl {
            return_if!(decl.search(searcher));
        }
        self.statements.search(searcher)
    }
}
impl<T> Search<T> for LabeledConcurrentStatement {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_labeled_concurrent_statement(self)
            .or_else(|| match self.statement {
                ConcurrentStatement::Block(ref block) => {
                    return_if!(block.decl.search(searcher));
                    block.statements.search(searcher)
                }
                ConcurrentStatement::Process(ref process) => {
                    return_if!(process.decl.search(searcher));
                    process.statements.search(searcher)
                }
                ConcurrentStatement::ForGenerate(ref gen) => gen.body.search(searcher),
                ConcurrentStatement::IfGenerate(ref gen) => {
                    for conditional in gen.conditionals.iter() {
                        return_if!(conditional.item.search(searcher));
                    }
                    if let Some(ref else_item) = gen.else_item {
                        else_item.search(searcher)
                    } else {
                        NotFound
                    }
                }
                ConcurrentStatement::CaseGenerate(ref gen) => {
                    for alternative in gen.alternatives.iter() {
                        return_if!(alternative.item.search(searcher))
                    }
                    NotFound
                }
                // @TODO not searched
                _ => NotFound,
            })
    }
}

impl<T> Search<T> for WithRef<Designator> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_designator_ref(self).or_else(|| NotFound)
    }
}

impl<T, U: Search<T>> Search<T> for WithPos<U> {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_with_pos(&self.pos)
            .or_else(|| self.item.search(searcher))
    }
}

impl<T> Search<T> for SelectedName {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        match self {
            SelectedName::Selected(prefix, designator) => {
                return_if!(prefix.search(searcher));
                return_if!(designator.search(searcher));
                NotFound
            }
            SelectedName::Designator(designator) => {
                searcher.search_designator_ref(designator).or_not_found()
            }
        }
    }
}

impl<T> Search<T> for SubtypeIndication {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_subtype_indication(&self).or_else(|| {
            return_if!(self.type_mark.search(searcher));
            NotFound
        })
    }
}

impl<T> Search<T> for TypeDeclaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        match self.def {
            TypeDefinition::ProtectedBody(ref body) => {
                return_if!(body.decl.search(searcher));
            }
            TypeDefinition::Protected(ref prot_decl) => {
                for item in prot_decl.items.iter() {
                    match item {
                        ProtectedTypeDeclarativeItem::Subprogram(ref subprogram) => {
                            return_if!(subprogram.search(searcher));
                        }
                    }
                }
            }
            TypeDefinition::Record(ref element_decls) => {
                for elem in element_decls {
                    return_if!(elem.subtype.search(searcher));
                }
            }
            TypeDefinition::Access(ref subtype_indication) => {
                return_if!(subtype_indication.search(searcher));
            }
            TypeDefinition::Array(.., ref subtype_indication) => {
                return_if!(subtype_indication.search(searcher));
            }
            TypeDefinition::Subtype(ref subtype_indication) => {
                return_if!(subtype_indication.search(searcher));
            }

            _ => {}
        }
        NotFound
    }
}

impl<T> Search<T> for Declaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_declaration(self).or_else(|| {
            match self {
                Declaration::Object(object) => {
                    return_if!(object.subtype_indication.search(searcher))
                }
                Declaration::Type(typ) => return_if!(typ.search(searcher)),
                Declaration::SubprogramBody(body) => {
                    return_if!(body.specification.search(searcher));
                    return_if!(body.declarations.search(searcher));
                }
                Declaration::SubprogramDeclaration(decl) => {
                    return_if!(decl.search(searcher));
                }
                _ => {}
            }
            NotFound
        })
    }
}

impl<T> Search<T> for InterfaceDeclaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_interface_declaration(self).or_else(|| {
            match self {
                InterfaceDeclaration::Object(ref decl) => {
                    return_if!(decl.subtype_indication.search(searcher));
                }
                InterfaceDeclaration::Subprogram(ref decl, _) => {
                    return_if!(decl.search(searcher));
                }
                _ => {}
            };
            NotFound
        })
    }
}

impl<T> Search<T> for SubprogramDeclaration {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        match self {
            SubprogramDeclaration::Function(ref decl) => return_if!(decl.search(searcher)),
            SubprogramDeclaration::Procedure(ref decl) => return_if!(decl.search(searcher)),
        }
        NotFound
    }
}

impl<T> Search<T> for ProcedureSpecification {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        self.parameter_list.search(searcher)
    }
}

impl<T> Search<T> for FunctionSpecification {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        return_if!(self.parameter_list.search(searcher));
        self.return_type.search(searcher)
    }
}

impl<T> Search<T> for EntityUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_source(self.source()).or_else(|| {
            return_if!(self.unit.generic_clause.search(searcher));
            return_if!(self.unit.port_clause.search(searcher));
            return_if!(self.unit.decl.search(searcher));
            return_if!(self.unit.statements.search(searcher));
            self.unit.decl.search(searcher)
        })
    }
}

impl<T> Search<T> for ArchitectureUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_source(self.source()).or_else(|| {
            return_if!(self.unit.decl.search(searcher));
            return_if!(self.unit.statements.search(searcher));
            self.unit.decl.search(searcher)
        })
    }
}

impl<T> Search<T> for PackageUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher.search_source(self.source()).or_else(|| {
            return_if!(self.unit.generic_clause.search(searcher));
            self.unit.decl.search(searcher)
        })
    }
}

impl<T> Search<T> for PackageBodyUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_source(self.source())
            .or_else(|| self.unit.decl.search(searcher))
    }
}

impl<T> Search<T> for PackageInstanceUnit {
    fn search(&self, searcher: &mut impl Searcher<T>) -> SearchResult<T> {
        searcher
            .search_source(self.source())
            .or_else(|| self.unit.package_name.search(searcher))
    }
}

pub struct ReferenceSearcher {
    source: Source,
    cursor: usize,
}

impl ReferenceSearcher {
    pub fn new(source: &Source, cursor: usize) -> ReferenceSearcher {
        ReferenceSearcher {
            source: source.clone(),
            cursor,
        }
    }
}

impl Searcher<SrcPos> for ReferenceSearcher {
    fn search_with_pos(&mut self, pos: &SrcPos) -> SearchState<SrcPos> {
        // cursor is the gap between character cursor and cursor + 1
        // Thus cursor will match character cursor and cursor + 1
        if pos.start <= self.cursor && self.cursor <= pos.end() {
            NotFinished
        } else {
            Finished(NotFound)
        }
    }

    fn search_designator_ref(&mut self, designator: &WithRef<Designator>) -> SearchState<SrcPos> {
        if let Some(ref reference) = designator.reference {
            Finished(Found(reference.clone()))
        } else {
            Finished(NotFound)
        }
    }

    fn search_source(&mut self, source: &Source) -> SearchState<SrcPos> {
        if source == &self.source {
            NotFinished
        } else {
            Finished(NotFound)
        }
    }
}