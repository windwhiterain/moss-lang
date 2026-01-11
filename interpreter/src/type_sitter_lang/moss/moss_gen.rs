#[doc = "Typed node `assign`\n\nThis node has these fields:\n\n- `key`: `name` ([`Name`])\n- `value`: `value` ([`Value`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Assign<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Assign<'tree> {
    #[doc = "Get the field `key`.\n\nThis child has type `name` ([`Name`])"]
    #[inline]
    pub fn key(&self) -> ::type_sitter::NodeResult<'tree, Name<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("key")
            .map(<Name<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
    #[doc = "Get the field `value`.\n\nThis child has type `value` ([`Value`])"]
    #[inline]
    pub fn value(&self) -> ::type_sitter::NodeResult<'tree, Value<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("value")
            .map(<Value<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Assign<'tree> {
    type WithLifetime<'a> = Assign<'a>;
    const KIND: &'static str = "assign";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "assign" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "assign");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `bracket`\n\nThis node has these fields:\n\n- `value`: `value` ([`Value`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Bracket<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Bracket<'tree> {
    #[doc = "Get the field `value`.\n\nThis child has type `value` ([`Value`])"]
    #[inline]
    pub fn value(&self) -> ::type_sitter::NodeResult<'tree, Value<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("value")
            .map(<Value<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Bracket<'tree> {
    type WithLifetime<'a> = Bracket<'a>;
    const KIND: &'static str = "bracket";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "bracket" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "bracket");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `call`\n\nThis node has these fields:\n\n- `func`: `value` ([`Value`])\n- `param`: `value` ([`Value`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Call<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Call<'tree> {
    #[doc = "Get the field `func`.\n\nThis child has type `value` ([`Value`])"]
    #[inline]
    pub fn func(&self) -> ::type_sitter::NodeResult<'tree, Value<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("func")
            .map(<Value<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
    #[doc = "Get the field `param`.\n\nThis child has type `value` ([`Value`])"]
    #[inline]
    pub fn param(&self) -> ::type_sitter::NodeResult<'tree, Value<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("param")
            .map(<Value<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Call<'tree> {
    type WithLifetime<'a> = Call<'a>;
    const KIND: &'static str = "call";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "call" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "call");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `dict`\n\nThis node has these fields:\n\n- `pair`: `pair*` ([`Pair`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Dict<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Dict<'tree> {
    #[doc = "Get the children of field `pair`.\n\nThese children have type `pair*` ([`Pair`])"]
    #[inline]
    pub fn pairs<'a>(
        &self,
        c: &'a mut ::type_sitter::TreeCursor<'tree>,
    ) -> impl ::std::iter::Iterator<Item = ::type_sitter::NodeResult<'tree, Pair<'tree>>> + 'a {
        ::type_sitter::Node::raw(self)
            .children_by_field_name("pair", &mut c.0)
            .map(<Pair<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Dict<'tree> {
    type WithLifetime<'a> = Dict<'a>;
    const KIND: &'static str = "dict";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "dict" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "dict");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `find`\n\nThis node has these fields:\n\n- `name`: `name` ([`Name`])\n- `value`: `value` ([`Value`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Find<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Find<'tree> {
    #[doc = "Get the field `name`.\n\nThis child has type `name` ([`Name`])"]
    #[inline]
    pub fn name(&self) -> ::type_sitter::NodeResult<'tree, Name<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("name")
            .map(<Name<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
    #[doc = "Get the field `value`.\n\nThis child has type `value` ([`Value`])"]
    #[inline]
    pub fn value(&self) -> ::type_sitter::NodeResult<'tree, Value<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("value")
            .map(<Value<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Find<'tree> {
    type WithLifetime<'a> = Find<'a>;
    const KIND: &'static str = "find";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "find" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "find");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `find_meta`\n\nThis node has these fields:\n\n- `name`: `name` ([`Name`])\n- `value`: `value` ([`Value`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct FindMeta<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> FindMeta<'tree> {
    #[doc = "Get the field `name`.\n\nThis child has type `name` ([`Name`])"]
    #[inline]
    pub fn name(&self) -> ::type_sitter::NodeResult<'tree, Name<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("name")
            .map(<Name<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
    #[doc = "Get the field `value`.\n\nThis child has type `value` ([`Value`])"]
    #[inline]
    pub fn value(&self) -> ::type_sitter::NodeResult<'tree, Value<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("value")
            .map(<Value<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for FindMeta<'tree> {
    type WithLifetime<'a> = FindMeta<'a>;
    const KIND: &'static str = "find_meta";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "find_meta" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "find_meta");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `int`\n\nThis node has no named children\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Int<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Int<'tree> {}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Int<'tree> {
    type WithLifetime<'a> = Int<'a>;
    const KIND: &'static str = "int";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "int" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "int");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `meta`\n\nThis node has these fields:\n\n- `name`: `name` ([`Name`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Meta<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Meta<'tree> {
    #[doc = "Get the field `name`.\n\nThis child has type `name` ([`Name`])"]
    #[inline]
    pub fn name(&self) -> ::type_sitter::NodeResult<'tree, Name<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("name")
            .map(<Name<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Meta<'tree> {
    type WithLifetime<'a> = Meta<'a>;
    const KIND: &'static str = "meta";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "meta" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "meta");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `name`\n\nThis node has no named children\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Name<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Name<'tree> {}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Name<'tree> {
    type WithLifetime<'a> = Name<'a>;
    const KIND: &'static str = "name";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "name" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "name");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `pair`\n\nThis node has these fields:\n\n- `key`: `name` ([`Name`])\n- `value`: `value` ([`Value`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Pair<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Pair<'tree> {
    #[doc = "Get the field `key`.\n\nThis child has type `name` ([`Name`])"]
    #[inline]
    pub fn key(&self) -> ::type_sitter::NodeResult<'tree, Name<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("key")
            .map(<Name<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
    #[doc = "Get the field `value`.\n\nThis child has type `value` ([`Value`])"]
    #[inline]
    pub fn value(&self) -> ::type_sitter::NodeResult<'tree, Value<'tree>> {
        ::type_sitter::Node::raw(self)
            .child_by_field_name("value")
            .map(<Value<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
            .expect(
                "required child not present, there should at least be a MISSING node in its place",
            )
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Pair<'tree> {
    type WithLifetime<'a> = Pair<'a>;
    const KIND: &'static str = "pair";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "pair" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "pair");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `scope`\n\nThis node has these fields:\n\n- `assign`: `assign*` ([`Assign`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Scope<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Scope<'tree> {
    #[doc = "Get the children of field `assign`.\n\nThese children have type `assign*` ([`Assign`])"]
    #[inline]
    pub fn assigns<'a>(
        &self,
        c: &'a mut ::type_sitter::TreeCursor<'tree>,
    ) -> impl ::std::iter::Iterator<Item = ::type_sitter::NodeResult<'tree, Assign<'tree>>> + 'a
    {
        ::type_sitter::Node::raw(self)
            .children_by_field_name("assign", &mut c.0)
            .map(<Assign<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Scope<'tree> {
    type WithLifetime<'a> = Scope<'a>;
    const KIND: &'static str = "scope";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "scope" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "scope");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `set`\n\nThis node has these fields:\n\n- `key`: `name*` ([`Name`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Set<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Set<'tree> {
    #[doc = "Get the children of field `key`.\n\nThese children have type `name*` ([`Name`])"]
    #[inline]
    pub fn keys<'a>(
        &self,
        c: &'a mut ::type_sitter::TreeCursor<'tree>,
    ) -> impl ::std::iter::Iterator<Item = ::type_sitter::NodeResult<'tree, Name<'tree>>> + 'a {
        ::type_sitter::Node::raw(self)
            .children_by_field_name("key", &mut c.0)
            .map(<Name<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Set<'tree> {
    type WithLifetime<'a> = Set<'a>;
    const KIND: &'static str = "set";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "set" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "set");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `source_file`\n\nThis node has these fields:\n\n- `assign`: `assign*` ([`Assign`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct SourceFile<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> SourceFile<'tree> {
    #[doc = "Get the children of field `assign`.\n\nThese children have type `assign*` ([`Assign`])"]
    #[inline]
    pub fn assigns<'a>(
        &self,
        c: &'a mut ::type_sitter::TreeCursor<'tree>,
    ) -> impl ::std::iter::Iterator<Item = ::type_sitter::NodeResult<'tree, Assign<'tree>>> + 'a
    {
        ::type_sitter::Node::raw(self)
            .children_by_field_name("assign", &mut c.0)
            .map(<Assign<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for SourceFile<'tree> {
    type WithLifetime<'a> = SourceFile<'a>;
    const KIND: &'static str = "source_file";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "source_file" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "source_file");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `string`\n\nThis node has these fields:\n\n- `content`: `string_content*` ([`StringContent`])\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct String<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> String<'tree> {
    #[doc = "Get the children of field `content`.\n\nThese children have type `string_content*` ([`StringContent`])"]
    #[inline]
    pub fn contents<'a>(
        &self,
        c: &'a mut ::type_sitter::TreeCursor<'tree>,
    ) -> impl ::std::iter::Iterator<Item = ::type_sitter::NodeResult<'tree, StringContent<'tree>>> + 'a
    {
        ::type_sitter::Node::raw(self)
            .children_by_field_name("content", &mut c.0)
            .map(<StringContent<'tree> as ::type_sitter::Node<'tree>>::try_from_raw)
    }
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for String<'tree> {
    type WithLifetime<'a> = String<'a>;
    const KIND: &'static str = "string";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "string" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "string");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `string_content`\n\nThis node has a named child of type `{string_escape | string_raw}`:\n\n- [`StringEscape`]\n- [`StringRaw`]\n\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct StringContent<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> StringContent<'tree> {}
#[automatically_derived]
impl<'tree> ::type_sitter::HasChild<'tree> for StringContent<'tree> {
    type Child = anon_unions::StringEscape_StringRaw<'tree>;
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for StringContent<'tree> {
    type WithLifetime<'a> = StringContent<'a>;
    const KIND: &'static str = "string_content";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "string_content" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "string_content");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `string_escape`\n\nThis node has no named children\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct StringEscape<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> StringEscape<'tree> {}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for StringEscape<'tree> {
    type WithLifetime<'a> = StringEscape<'a>;
    const KIND: &'static str = "string_escape";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "string_escape" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "string_escape");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `string_raw`\n\nThis node has no named children\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct StringRaw<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> StringRaw<'tree> {}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for StringRaw<'tree> {
    type WithLifetime<'a> = StringRaw<'a>;
    const KIND: &'static str = "string_raw";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "string_raw" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "string_raw");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
#[doc = "Typed node `value`\n\nThis node has a named child of type `{bracket | call | dict | find | find_meta | int | meta | name | scope | set | string}`:\n\n- [`Bracket`]\n- [`Call`]\n- [`Dict`]\n- [`Find`]\n- [`FindMeta`]\n- [`Int`]\n- [`Meta`]\n- [`Name`]\n- [`Scope`]\n- [`Set`]\n- [`String`]\n\n"]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[allow(non_camel_case_types)]
pub struct Value<'tree>(::type_sitter::raw::Node<'tree>);
#[automatically_derived]
#[allow(unused)]
impl<'tree> Value<'tree> {}
#[automatically_derived]
impl<'tree> ::type_sitter::HasChild<'tree> for Value<'tree> {
    type Child = anon_unions::Bracket_Call_Dict_Find_FindMeta_Int_Meta_Name_Scope_Set_String<'tree>;
}
#[automatically_derived]
impl<'tree> ::type_sitter::Node<'tree> for Value<'tree> {
    type WithLifetime<'a> = Value<'a>;
    const KIND: &'static str = "value";
    #[inline]
    fn try_from_raw(
        node: ::type_sitter::raw::Node<'tree>,
    ) -> ::type_sitter::NodeResult<'tree, Self> {
        if node.kind() == "value" {
            Ok(Self(node))
        } else {
            Err(::type_sitter::IncorrectKind::new::<Self>(node))
        }
    }
    #[inline]
    unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
        debug_assert_eq!(node.kind(), "value");
        Self(node)
    }
    #[inline]
    fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
        &self.0
    }
    #[inline]
    fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
        &mut self.0
    }
    #[inline]
    fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
        self.0
    }
}
pub mod symbols {
    #[allow(unused_imports)]
    use super::*;
    #[doc = "Typed node `\"`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct DoubleQuote<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> DoubleQuote<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for DoubleQuote<'tree> {
        type WithLifetime<'a> = DoubleQuote<'a>;
        const KIND: &'static str = "\"";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "\"" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), "\"");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `(`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct LParen<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> LParen<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for LParen<'tree> {
        type WithLifetime<'a> = LParen<'a>;
        const KIND: &'static str = "(";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "(" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), "(");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `)`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct RParen<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> RParen<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for RParen<'tree> {
        type WithLifetime<'a> = RParen<'a>;
        const KIND: &'static str = ")";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == ")" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), ")");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `,`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct Comma<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> Comma<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for Comma<'tree> {
        type WithLifetime<'a> = Comma<'a>;
        const KIND: &'static str = ",";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "," {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), ",");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `.`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct Dot<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> Dot<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for Dot<'tree> {
        type WithLifetime<'a> = Dot<'a>;
        const KIND: &'static str = ".";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "." {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), ".");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `.@`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct DotAt<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> DotAt<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for DotAt<'tree> {
        type WithLifetime<'a> = DotAt<'a>;
        const KIND: &'static str = ".@";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == ".@" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), ".@");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `:`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct Colon<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> Colon<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for Colon<'tree> {
        type WithLifetime<'a> = Colon<'a>;
        const KIND: &'static str = ":";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == ":" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), ":");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `;`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct Semicolon<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> Semicolon<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for Semicolon<'tree> {
        type WithLifetime<'a> = Semicolon<'a>;
        const KIND: &'static str = ";";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == ";" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), ";");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `=`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct Eq<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> Eq<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for Eq<'tree> {
        type WithLifetime<'a> = Eq<'a>;
        const KIND: &'static str = "=";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "=" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), "=");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `@`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct At<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> At<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for At<'tree> {
        type WithLifetime<'a> = At<'a>;
        const KIND: &'static str = "@";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "@" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), "@");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `{`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct LBrace<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> LBrace<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for LBrace<'tree> {
        type WithLifetime<'a> = LBrace<'a>;
        const KIND: &'static str = "{";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "{" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), "{");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
    #[doc = "Typed node `}`\n\nThis node has no named children\n"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    #[allow(non_camel_case_types)]
    pub struct RBrace<'tree>(::type_sitter::raw::Node<'tree>);
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> RBrace<'tree> {}
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for RBrace<'tree> {
        type WithLifetime<'a> = RBrace<'a>;
        const KIND: &'static str = "}";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            if node.kind() == "}" {
                Ok(Self(node))
            } else {
                Err(::type_sitter::IncorrectKind::new::<Self>(node))
            }
        }
        #[inline]
        unsafe fn from_raw_unchecked(node: ::type_sitter::raw::Node<'tree>) -> Self {
            debug_assert_eq!(node.kind(), "}");
            Self(node)
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            &self.0
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            &mut self.0
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            self.0
        }
    }
}
pub mod anon_unions {
    #[allow(unused_imports)]
    use super::*;
    #[doc = "One of `{bracket | call | dict | find | find_meta | int | meta | name | scope | set | string}`:\n- [`Bracket`]\n- [`Call`]\n- [`Dict`]\n- [`Find`]\n- [`FindMeta`]\n- [`Int`]\n- [`Meta`]\n- [`Name`]\n- [`Scope`]\n- [`Set`]\n- [`String`]"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[allow(non_camel_case_types)]
    pub enum Bracket_Call_Dict_Find_FindMeta_Int_Meta_Name_Scope_Set_String<'tree> {
        Bracket(Bracket<'tree>),
        Call(Call<'tree>),
        Dict(Dict<'tree>),
        Find(Find<'tree>),
        FindMeta(FindMeta<'tree>),
        Int(Int<'tree>),
        Meta(Meta<'tree>),
        Name(Name<'tree>),
        Scope(Scope<'tree>),
        Set(Set<'tree>),
        String(String<'tree>),
    }
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> Bracket_Call_Dict_Find_FindMeta_Int_Meta_Name_Scope_Set_String<'tree> {
        #[doc = "Returns the node if it is of type `bracket` ([`Bracket`]), otherwise returns `None`"]
        #[inline]
        pub fn as_bracket(self) -> ::std::option::Option<Bracket<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Bracket(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `call` ([`Call`]), otherwise returns `None`"]
        #[inline]
        pub fn as_call(self) -> ::std::option::Option<Call<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Call(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `dict` ([`Dict`]), otherwise returns `None`"]
        #[inline]
        pub fn as_dict(self) -> ::std::option::Option<Dict<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Dict(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `find` ([`Find`]), otherwise returns `None`"]
        #[inline]
        pub fn as_find(self) -> ::std::option::Option<Find<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Find(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `find_meta` ([`FindMeta`]), otherwise returns `None`"]
        #[inline]
        pub fn as_find_meta(self) -> ::std::option::Option<FindMeta<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::FindMeta(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `int` ([`Int`]), otherwise returns `None`"]
        #[inline]
        pub fn as_int(self) -> ::std::option::Option<Int<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Int(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `meta` ([`Meta`]), otherwise returns `None`"]
        #[inline]
        pub fn as_meta(self) -> ::std::option::Option<Meta<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Meta(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `name` ([`Name`]), otherwise returns `None`"]
        #[inline]
        pub fn as_name(self) -> ::std::option::Option<Name<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Name(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `scope` ([`Scope`]), otherwise returns `None`"]
        #[inline]
        pub fn as_scope(self) -> ::std::option::Option<Scope<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Scope(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `set` ([`Set`]), otherwise returns `None`"]
        #[inline]
        pub fn as_set(self) -> ::std::option::Option<Set<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::Set(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `string` ([`String`]), otherwise returns `None`"]
        #[inline]
        pub fn as_string(self) -> ::std::option::Option<String<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::String(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
    }
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree>
        for Bracket_Call_Dict_Find_FindMeta_Int_Meta_Name_Scope_Set_String<'tree>
    {
        type WithLifetime<'a> = Bracket_Call_Dict_Find_FindMeta_Int_Meta_Name_Scope_Set_String<'a>;
        const KIND: &'static str =
            "{bracket | call | dict | find | find_meta | int | meta | name | scope | set | string}";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            match node.kind() {
                "bracket" => Ok(unsafe {
                    Self::Bracket(
                        <Bracket<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "call" => Ok(unsafe {
                    Self::Call(
                        <Call<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "dict" => Ok(unsafe {
                    Self::Dict(
                        <Dict<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "find" => Ok(unsafe {
                    Self::Find(
                        <Find<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "find_meta" => Ok(unsafe {
                    Self::FindMeta(
                        <FindMeta<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "int" => Ok(unsafe {
                    Self::Int(<Int<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node))
                }),
                "meta" => Ok(unsafe {
                    Self::Meta(
                        <Meta<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "name" => Ok(unsafe {
                    Self::Name(
                        <Name<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "scope" => Ok(unsafe {
                    Self::Scope(
                        <Scope<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                "set" => Ok(unsafe {
                    Self::Set(<Set<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node))
                }),
                "string" => Ok(unsafe {
                    Self::String(
                        <String<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                _ => Err(::type_sitter::IncorrectKind::new::<Self>(node)),
            }
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            match self {
                Self::Bracket(x) => ::type_sitter::Node::raw(x),
                Self::Call(x) => ::type_sitter::Node::raw(x),
                Self::Dict(x) => ::type_sitter::Node::raw(x),
                Self::Find(x) => ::type_sitter::Node::raw(x),
                Self::FindMeta(x) => ::type_sitter::Node::raw(x),
                Self::Int(x) => ::type_sitter::Node::raw(x),
                Self::Meta(x) => ::type_sitter::Node::raw(x),
                Self::Name(x) => ::type_sitter::Node::raw(x),
                Self::Scope(x) => ::type_sitter::Node::raw(x),
                Self::Set(x) => ::type_sitter::Node::raw(x),
                Self::String(x) => ::type_sitter::Node::raw(x),
            }
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            match self {
                Self::Bracket(x) => ::type_sitter::Node::raw_mut(x),
                Self::Call(x) => ::type_sitter::Node::raw_mut(x),
                Self::Dict(x) => ::type_sitter::Node::raw_mut(x),
                Self::Find(x) => ::type_sitter::Node::raw_mut(x),
                Self::FindMeta(x) => ::type_sitter::Node::raw_mut(x),
                Self::Int(x) => ::type_sitter::Node::raw_mut(x),
                Self::Meta(x) => ::type_sitter::Node::raw_mut(x),
                Self::Name(x) => ::type_sitter::Node::raw_mut(x),
                Self::Scope(x) => ::type_sitter::Node::raw_mut(x),
                Self::Set(x) => ::type_sitter::Node::raw_mut(x),
                Self::String(x) => ::type_sitter::Node::raw_mut(x),
            }
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            match self {
                Self::Bracket(x) => x.into_raw(),
                Self::Call(x) => x.into_raw(),
                Self::Dict(x) => x.into_raw(),
                Self::Find(x) => x.into_raw(),
                Self::FindMeta(x) => x.into_raw(),
                Self::Int(x) => x.into_raw(),
                Self::Meta(x) => x.into_raw(),
                Self::Name(x) => x.into_raw(),
                Self::Scope(x) => x.into_raw(),
                Self::Set(x) => x.into_raw(),
                Self::String(x) => x.into_raw(),
            }
        }
    }
    #[doc = "One of `{string_escape | string_raw}`:\n- [`StringEscape`]\n- [`StringRaw`]"]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[allow(non_camel_case_types)]
    pub enum StringEscape_StringRaw<'tree> {
        StringEscape(StringEscape<'tree>),
        StringRaw(StringRaw<'tree>),
    }
    #[automatically_derived]
    #[allow(unused)]
    impl<'tree> StringEscape_StringRaw<'tree> {
        #[doc = "Returns the node if it is of type `string_escape` ([`StringEscape`]), otherwise returns `None`"]
        #[inline]
        pub fn as_string_escape(self) -> ::std::option::Option<StringEscape<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::StringEscape(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
        #[doc = "Returns the node if it is of type `string_raw` ([`StringRaw`]), otherwise returns `None`"]
        #[inline]
        pub fn as_string_raw(self) -> ::std::option::Option<StringRaw<'tree>> {
            #[allow(irrefutable_let_patterns)]
            if let Self::StringRaw(x) = self {
                ::std::option::Option::Some(x)
            } else {
                ::std::option::Option::None
            }
        }
    }
    #[automatically_derived]
    impl<'tree> ::type_sitter::Node<'tree> for StringEscape_StringRaw<'tree> {
        type WithLifetime<'a> = StringEscape_StringRaw<'a>;
        const KIND: &'static str = "{string_escape | string_raw}";
        #[inline]
        fn try_from_raw(
            node: ::type_sitter::raw::Node<'tree>,
        ) -> ::type_sitter::NodeResult<'tree, Self> {
            match node.kind() {
                "string_escape" => Ok(unsafe {
                    Self::StringEscape(
                        <StringEscape<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(
                            node,
                        ),
                    )
                }),
                "string_raw" => Ok(unsafe {
                    Self::StringRaw(
                        <StringRaw<'tree> as ::type_sitter::Node<'tree>>::from_raw_unchecked(node),
                    )
                }),
                _ => Err(::type_sitter::IncorrectKind::new::<Self>(node)),
            }
        }
        #[inline]
        fn raw(&self) -> &::type_sitter::raw::Node<'tree> {
            match self {
                Self::StringEscape(x) => ::type_sitter::Node::raw(x),
                Self::StringRaw(x) => ::type_sitter::Node::raw(x),
            }
        }
        #[inline]
        fn raw_mut(&mut self) -> &mut ::type_sitter::raw::Node<'tree> {
            match self {
                Self::StringEscape(x) => ::type_sitter::Node::raw_mut(x),
                Self::StringRaw(x) => ::type_sitter::Node::raw_mut(x),
            }
        }
        #[inline]
        fn into_raw(self) -> ::type_sitter::raw::Node<'tree> {
            match self {
                Self::StringEscape(x) => x.into_raw(),
                Self::StringRaw(x) => x.into_raw(),
            }
        }
    }
}
