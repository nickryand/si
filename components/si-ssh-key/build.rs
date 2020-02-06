fn main() {
    tonic_build::configure()
        .extern_path(".si.data", "::si_data::data")
        .extern_path(".si.account", "::si_account::protobuf")
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"camelCase\")]")
        .field_attribute(
            "key_type",
            "#[serde(serialize_with = \"crate::serde_enum::key_type_enum_s\", deserialize_with = \"crate::serde_enum::key_type_enum_d\")]",
        )
        .field_attribute(
            "key_format",
            "#[serde(serialize_with = \"crate::serde_enum::key_format_enum_s\", deserialize_with = \"crate::serde_enum::key_format_enum_d\")]",
        )
        .field_attribute("in", "#[serde(rename = \"in\")]")
        .compile(&["si-ssh-key/proto/si.ssh_key.proto"], &[".."])
        .unwrap();
}
