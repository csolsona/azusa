// Regenerate the schema with:
// cynic introspect https://graphql.anilist.co -o schemas/anilist.graphql
fn main() -> Result<(), Box<dyn std::error::Error>> {
	cynic_codegen::register_schema("anilist")
		.from_sdl_file("schemas/anilist.graphql")?
		.as_default()?;
	Ok(())
}
