pub mod server;
use std::error::Error;

pub use server::*;
pub mod todos;
use todos::todos_server::TodosServer;
pub use todos::*;
use tonic::transport::Server;

mod todos_proto {
    include!("todos.rs");

    pub(crate) const FILE_DESCRIPTOR: &[u8] =
        tonic::include_file_descriptor_set!("todos_descriptor");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(todos_proto::FILE_DESCRIPTOR)
        .build()
        .unwrap();

    let addr = "127.0.0.1:8000".parse()?;
    let inner = TodoService::default();

    println!("Server running on {addr}");

    Server::builder()
        .add_service(TodosServer::new(inner))
        .add_service(reflection_service)
        .serve(addr)
        .await?;

    Ok(())
}
