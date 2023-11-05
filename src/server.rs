use futures::Stream;
use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{Response, Status};

use crate::{todos::Todo, todos_server::Todos, TodoChangeResponse};

pub struct TodoService {
    todos: Arc<Mutex<HashMap<u32, Todo>>>,
}

impl Default for TodoService {
    fn default() -> Self {
        Self {
            todos: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[tonic::async_trait]
impl Todos for TodoService {
    #[doc = " Add to Todos"]
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    async fn add(
        &self,
        request: tonic::Request<super::Todo>,
    ) -> Result<Response<TodoChangeResponse>, Status> {
        let todo = request.into_inner();

        let identifier = match todo.id.clone() {
            Some(id) => id,
            None => return Err(Status::invalid_argument("Id missing")),
        };

        let mut map = self.todos.lock().await;

        match map.get(&identifier.id) {
            Some(_) => return Err(Status::already_exists("Todo with this Id already exists")),
            None => {
                map.insert(identifier.id, todo);
                return Ok(Response::new(TodoChangeResponse {
                    id: Some(identifier),
                    message: "New todo created".to_owned(),
                }));
            }
        }
    }

    #[doc = " Remove a todo"]
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    async fn remove(
        &self,
        request: tonic::Request<super::TodoIdentifier>,
    ) -> Result<Response<TodoChangeResponse>, Status> {
        let identifier = request.into_inner();
        let mut map = self.todos.lock().await;
        match map.get(&identifier.id) {
            Some(_) => {
                map.remove(&identifier.id);
                return Ok(Response::new(TodoChangeResponse {
                    id: Some(identifier),
                    message: "Removed a todo".to_owned(),
                }));
            }
            None => {
                return Err(Status::not_found("Corresponding todo not found"));
            }
        }
    }

    #[doc = " Update status of a Todo"]
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    async fn update_status(
        &self,
        request: tonic::Request<super::TodoStatusUpdateRequest>,
    ) -> Result<Response<TodoChangeResponse>, Status> {
        let request = request.into_inner();

        let mut map = self.todos.lock().await;

        let identifier = match request.id {
            Some(id) => id,
            None => return Err(Status::invalid_argument("Missing Id")),
        };
        match map.get_mut(&identifier.id) {
            Some(todo) => {
                todo.status = request.status;

                return Ok(Response::new(TodoChangeResponse {
                    id: Some(identifier),
                    message: "Updated status successfully".to_owned(),
                }));
            }
            None => return Err(Status::not_found("Corresponding todo not found")),
        }
    }

    #[doc = " Get a Todo by Identifier"]
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    async fn get(
        &self,
        request: tonic::Request<super::TodoIdentifier>,
    ) -> Result<Response<Todo>, Status> {
        let identifier = request.into_inner();
        let map = self.todos.lock().await;
        match map.get(&identifier.id) {
            Some(todo) => {
                return Ok(Response::new(todo.clone()));
            }
            None => {
                return Err(Status::not_found("Corresponding todo not found"));
            }
        }
    }

    #[doc = " Server streaming response type for the Watch method."]
    type WatchStream = Pin<Box<dyn Stream<Item = Result<Todo, Status>> + Send>>;

    #[doc = " Watches over a Todo by Identifier"]
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    async fn watch(
        &self,
        request: tonic::Request<super::TodoIdentifier>,
    ) -> Result<Response<Self::WatchStream>, Status> {
        let identifier = request.into_inner();
        let map = self.todos.lock().await;
        let mut previous_todo = match map.get(&identifier.id) {
            Some(todo) => todo.clone(),
            None => return Err(Status::not_found("Corresponding todo not found")),
        };

        let (tx, rx) = mpsc::unbounded_channel();

        let todos = self.todos.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                let map = todos.lock().await;

                let new_todo = match map.get(&identifier.id) {
                    Some(todo) => todo.clone(),
                    None => {
                        let _ = tx.send(Err(Status::not_found("Corresponding todo not found")));
                        return;
                    }
                };

                if new_todo != previous_todo {
                    let _ = tx.send(Ok(new_todo.clone()));
                    previous_todo = new_todo;
                }
            }
        });

        let stream = UnboundedReceiverStream::new(rx);

        Ok(Response::new(Box::pin(stream) as Self::WatchStream))
    }
}
