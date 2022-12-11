use std::process::Command;

use tonic_build::Builder;

fn main() {
    tonic_build::configure()
        .out_dir("src/pb")
        .with_sql_type(&["reservation.ReservationStatus"])
        .with_builder(&["reservation.ReservationQuery", "reservation.ReservationFilter"])
        .with_builder_into(
            "reservation.ReservationQuery",
            &[
                "user_id",
                "resource_id",
                "page",
                "page_size",
                "desc",
                "status",
            ],
        )
        .with_builder_into(
            "reservation.ReservationFilter",
            &[
                "user_id",
                "resource_id",
                "cursor",
                "page_size",
                "desc",
                "status",
            ],
        )
        .with_builder_option("reservation.ReservationQuery", &["start", "end"])
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

trait BuilderExt {
    fn with_sql_type(self, paths: &[&str]) -> Self;
    fn with_builder(self, paths: &[&str]) -> Self;
    fn with_builder_into(self, path: &str, fields: &[&str]) -> Self;
    fn with_builder_option(self, path: &str, fields: &[&str]) -> Self;
}

impl BuilderExt for Builder {
    fn with_sql_type(self, paths: &[&str]) -> Self {
        paths.iter().fold(self, |builder, path| {
            builder.type_attribute(path, "#[derive(sqlx::Type)]")
        })
    }

    fn with_builder(self, paths: &[&str]) -> Self {
        paths.iter().fold(self, |builder, path| {
            builder.type_attribute(path, "#[derive(derive_builder::Builder)]")
        })
    }

    fn with_builder_into(self, path: &str, fields: &[&str]) -> Self {
        fields.iter().fold(self, |builder, field| {
            builder.field_attribute(
                format!("{path}.{field}"),
                "#[builder(setter(into), default)]",
            )
        })
    }

    fn with_builder_option(self, path: &str, fields: &[&str]) -> Self {
        fields.iter().fold(self, |builder, field| {
            builder.field_attribute(
                format!("{path}.{field}"),
                "#[builder(setter(into, strip_option))]",
            )
        })
    }
}
