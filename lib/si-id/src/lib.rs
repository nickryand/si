macro_rules! id {
    (
        $(#[$($attrss:tt)*])*
        $name:ident
    ) => {
        $(#[$($attrss)*])*
        #[allow(missing_docs)]
        #[derive(
            Eq,
            PartialEq,
            PartialOrd,
            Ord,
            Copy,
            Clone,
            Hash,
            Default,
            derive_more::From,
            derive_more::Into,
            derive_more::Display,
            serde::Serialize,
            serde::Deserialize,
        )]
        pub struct $name(::ulid::Ulid);

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($name)).field(&self.0.to_string()).finish()
            }
        }

        impl $name {
            /// Length of a string-encoded ID in bytes.
            pub const ID_LEN: usize = ::ulid::ULID_LEN;

            /// Generates a new key which is virtually guaranteed to be unique.
            pub fn generate() -> Self {
                Self(::ulid::Ulid::new())
            }

            pub fn new() -> Self {
                Self::generate()
            }

            pub fn array_to_str<'buf>(&self, buf: &'buf mut [u8; ::ulid::ULID_LEN]) -> &'buf mut str {
                self.0.array_to_str(buf)
            }

            pub fn array_to_str_buf() -> [u8; ::ulid::ULID_LEN] {
                [0; ::ulid::ULID_LEN]
            }

            /// Constructs a new instance of Self from the given raw identifier.
            ///
            /// This function is typically used to consume ownership of the specified identifier.
            pub fn from_raw_id(value: ::ulid::Ulid) -> Self {
                Self(value)
            }

            /// Extracts the raw identifier.
            ///
            /// This function is typically used to borrow an owned idenfier.
            pub fn as_raw_id(&self) -> ::ulid::Ulid {
                self.0
            }

            /// Consumes this object, returning the raw underlying identifier.
            ///
            /// This function is typically used to transfer ownership of the underlying identifier
            /// to the caller.
            pub fn into_raw_id(self) -> ::ulid::Ulid {
                self.0
            }
        }

        impl From<$name> for crate::ulid_wrapper::Ulid {
            fn from(pk: $name) -> Self {
                pk.0.into()
            }
        }

        impl<'a> From<&'a $name> for crate::ulid_wrapper::Ulid {
            fn from(pk: &'a $name) -> Self {
                pk.0.into()
            }
        }

        impl From<crate::ulid_wrapper::Ulid> for $name {
            fn from(ulid: crate::ulid_wrapper::Ulid) -> Self {
                ulid.inner().into()
            }
        }

        impl From<$name> for String {
            fn from(id: $name) -> Self {
                ulid::Ulid::from(id.0).into()
            }
        }

        impl<'a> From<&'a $name> for $name {
            fn from(id: &'a $name) -> Self {
                *id
            }
        }

        impl std::str::FromStr for $name {
            type Err = ::ulid::DecodeError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Ok(Self(::ulid::Ulid::from_string(s)?))
            }
        }

        impl<'a> postgres_types::FromSql<'a> for $name {
            fn from_sql(
                ty: &postgres_types::Type,
                raw: &'a [u8],
            ) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
                let id: String = postgres_types::FromSql::from_sql(ty, raw)?;
                Ok(Self(::ulid::Ulid::from_string(&id)?))
            }

            fn accepts(ty: &postgres_types::Type) -> bool {
                ty == &postgres_types::Type::BPCHAR
                    || ty.kind() == &postgres_types::Kind::Domain(postgres_types::Type::BPCHAR)
            }
        }

        impl postgres_types::ToSql for $name {
            fn to_sql(
                &self,
                ty: &postgres_types::Type,
                out: &mut postgres_types::private::BytesMut,
            ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>>
            where
                Self: Sized,
            {
                postgres_types::ToSql::to_sql(&self.0.to_string(), ty, out)
            }

            fn accepts(ty: &postgres_types::Type) -> bool
            where
                Self: Sized,
            {
                ty == &postgres_types::Type::BPCHAR
                    || ty.kind() == &postgres_types::Kind::Domain(postgres_types::Type::BPCHAR)
            }

            fn to_sql_checked(
                &self,
                ty: &postgres_types::Type,
                out: &mut postgres_types::private::BytesMut,
            ) -> Result<postgres_types::IsNull, Box<dyn std::error::Error + Sync + Send>> {
                postgres_types::ToSql::to_sql(&self.0.to_string(), ty, out)
            }
        }
    };
}

pub mod ulid_wrapper;

/// This module contains IDs that are no longer in use, but are needed for backwards compatibility.
pub mod deprecated {
    id!(FuncExecutionPk);
}

/// The forbidden function of doom.
pub fn nil() -> ulid::Ulid {
    ulid::Ulid::nil()
}

id!(ActionId);
id!(ActionPrototypeId);
id!(ChangeSetId);
id!(AttributePrototypeArgumentId);
id!(AttributePrototypeId);
id!(AttributeValueId);
id!(AuthenticationPrototypeId);
id!(ComponentId);
id!(FuncArgumentId);
id!(FuncId);
id!(SecretId);
id!(FuncRunId);
id!(CachedModuleId);
id!(KeyPairPk);
id!(ChangeSetId);
id!(SchemaVariantId);
id!(WorkspacePk);
id!(WorkspaceId);
id!(UserPk);
id!(ModuleId);
id!(InputSocketId);
id!(UserPk);
id!(OutputSocketId);
id!(PropId);
id!(DeprecatedVectorClockId);
id!(SchemaId);
id!(ValidationOutputId);
id!(StaticArgumentValueId);
id!(HistoryEventPk);
