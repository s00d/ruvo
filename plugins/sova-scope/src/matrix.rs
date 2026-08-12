//! Declarative role matrix builder.

use crate::types::{Ability, Capabilities, ResourceRef};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourcePattern {
    All,
    Kind(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    Always,
    Owner,
}

#[derive(Clone, Debug)]
struct Rule {
    role: &'static str,
    ability: Ability,
    resource: ResourcePattern,
    condition: Condition,
    allow: bool,
}

#[derive(Clone, Debug, Default)]
pub struct RoleMatrix {
    rules: Vec<Rule>,
    allow_all_roles: Vec<&'static str>,
}

impl RoleMatrix {
    pub fn evaluate(
        &self,
        role: &str,
        ability: Ability,
        resource: Option<ResourceRef>,
        is_owner: bool,
    ) -> bool {
        if self.allow_all_roles.contains(&role) {
            return true;
        }

        let mut decided: Option<bool> = None;
        for rule in &self.rules {
            if rule.role != role || rule.ability != ability {
                continue;
            }
            if !resource_matches(rule.resource, resource) {
                continue;
            }
            let ok = match rule.condition {
                Condition::Always => true,
                Condition::Owner => is_owner,
            };
            if ok {
                decided = Some(rule.allow);
            }
        }

        decided.unwrap_or(false)
    }

    pub fn capabilities(
        &self,
        role: &str,
        resource: Option<ResourceRef>,
        is_owner: bool,
    ) -> Capabilities {
        Capabilities {
            view: self.evaluate(role, Ability::View, resource, is_owner),
            create: self.evaluate(role, Ability::Create, resource, is_owner),
            update: self.evaluate(role, Ability::Update, resource, is_owner),
            delete: self.evaluate(role, Ability::Delete, resource, is_owner),
            manage: self.evaluate(role, Ability::Manage, resource, is_owner),
        }
    }
}

fn resource_matches(pattern: ResourcePattern, resource: Option<ResourceRef>) -> bool {
    match pattern {
        ResourcePattern::All => true,
        ResourcePattern::Kind(kind) => resource.is_some_and(|r| r.kind == kind),
    }
}

#[derive(Default)]
pub struct MatrixBuilder {
    kinds: HashMap<&'static str, RoleMatrix>,
}

impl MatrixBuilder {
    pub fn allow_all(&mut self, scope_kind: &'static str, role: &'static str) -> &mut Self {
        self.kinds
            .entry(scope_kind)
            .or_default()
            .allow_all_roles
            .push(role);
        self
    }

    pub fn allow(
        &mut self,
        scope_kind: &'static str,
        role: &'static str,
        ability: Ability,
        resource: ResourcePattern,
        condition: Condition,
    ) -> &mut Self {
        self.kinds
            .entry(scope_kind)
            .or_default()
            .rules
            .push(Rule {
                role,
                ability,
                resource,
                condition,
                allow: true,
            });
        self
    }

    pub fn deny(
        &mut self,
        scope_kind: &'static str,
        role: &'static str,
        ability: Ability,
        resource: ResourcePattern,
        condition: Condition,
    ) -> &mut Self {
        self.kinds
            .entry(scope_kind)
            .or_default()
            .rules
            .push(Rule {
                role,
                ability,
                resource,
                condition,
                allow: false,
            });
        self
    }

    pub fn build(self) -> HashMap<&'static str, RoleMatrix> {
        self.kinds
    }
}

// Ergonomics aliases used in docs / atlas config.