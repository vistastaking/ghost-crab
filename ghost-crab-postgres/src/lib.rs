pub use ghost_crab_macros::StructToPGTable;

pub trait PostgresDDL {
    fn fields() -> Vec<(String, String)>;
    fn type_to_pg_type(ty: &str) -> &'static str;
    fn ddl() -> String;
    fn create_table(conn: &mut postgres::Client) -> Result<(), postgres::Error>;
}
