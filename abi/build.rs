use std::process::Command;

use proto_builder_trait::tonic::BuilderAttributes;
// use tonic_build::Builder;

fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .with_sqlx_type(&["reservation.ReservationStatus"])
        .with_derive_builder(&[
            "reservation.ReservationQuery",
            "reservation.ReservationFilter",
        ])
        .with_derive_builder_into(
            "reservation.ReservationQuery",
            &["user_id", "resource_id", "page", "desc", "status"],
        )
        .with_derive_builder_into(
            "reservation.ReservationFilter",
            &["user_id", "resource_id", "desc", "status"],
        )
        .with_derive_builder_option("reservation.ReservationFilter", &["cursor"])
        .with_derive_builder_option("reservation.ReservationQuery", &["start", "end"])
        .with_field_attributes(
            &["page_size"],
            &["#[builder(setter(into), default = \"10\")]"],
        )
        .with_type_attributes(
            &[
                "reservation.ReservationFilter",
                // "reservation.ReservationQuery",
            ],
            &[r#"#[builder(build_fn(name = "private_build"))]"#],
        )
        .compile(&["protos/reservation.proto"], &["protos"])
        .unwrap();

    // fs::remove_file("src/pb/google.protobuf.rs").unwrap();

    Command::new("cargo")
        .args(["fmt"])
        .current_dir("src/pb")
        .output()
        .unwrap();

    println!("cargo:rerun-if-changed=protos/reservation.proto");
}

// trait BuilderExt {
//     fn with_sql_type(self, paths: &[&str]) -> Self;
//     fn with_builder(self, paths: &[&str]) -> Self;
//     fn with_builder_into(self, path: &str, fields: &[&str]) -> Self;
//     fn with_builder_option(self, path: &str, fields: &[&str]) -> Self;
// }
// impl BuilderExt for Builder {
//     fn with_sql_type(self, paths: &[&str]) -> Self {
//         paths.iter().fold(self, |builder, path| {
//             builder.type_attribute(path, "#[derive(sqlx::Type)]")
//         })
//     }
//     fn with_builder(self, paths: &[&str]) -> Self {
//         paths.iter().fold(self, |builder, path| {
//             builder.type_attribute(path, "#[derive(derive_builder::Builder)]")
//         })
//     }
//     fn with_builder_into(self, path: &str, fields: &[&str]) -> Self {
//         fields.iter().fold(self, |builder, field| {
//             builder.field_attribute(
//                 format!("{path}.{field}"),
//                 "#[builder(setter(into), default)]",
//             )
//         })
//     }
//     fn with_builder_option(self, path: &str, fields: &[&str]) -> Self {
//         fields.iter().fold(self, |builder, field| {
//             builder.field_attribute(
//                 format!("{path}.{field}"),
//                 "#[builder(setter(into, strip_option))]",
//             )
//         })
//     }
// }
