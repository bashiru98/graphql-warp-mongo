use async_graphql::{Context, FieldResult, InputObject};
// use async_graphql::parser::Error;
use crate::{models::todo_model::Todo, repository::mongodb_repo::MongoRepo};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    EmptySubscription, Schema, SimpleObject,
};
use async_graphql_warp::{GraphQLBadRequest, GraphQLResponse};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use warp::{http::Response as HttpResponse, Filter, Rejection};

mod models;
mod repository;

// define a store for data sources (can be database, cache, etc)
struct Store {
    db: MongoRepo,
}

impl Store {
    async fn new() -> Self {
        Store {
            db: MongoRepo::init().await,
        }
    }
}

#[derive(SimpleObject, Serialize)]
pub struct Todoql {
    name: String,
    id: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(InputObject, Deserialize)]
pub struct TodoInput {
    id: Option<String>,
    name: String,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Serialize, SimpleObject)]
struct DeleteTodoresponse {
    deleted: bool,
}

pub struct Mutation;

#[async_graphql::Object]
impl Mutation {
    async fn create_todo (
        &self,
        ctx: &Context<'_>,
        new_todo: TodoInput,
    ) -> FieldResult<Todoql> {
        // this is to ensure that the db connection is always reused
        let store = ctx.data::<Store>()?;
        let todo = Todo {
            id: None,
            name: new_todo.name,
            created_at: new_todo.created_at,
            updated_at: new_todo.updated_at,
        };

        let todo = store.db.create_todo(todo).await?;
        Ok(Todoql {
            name: todo.name,
            id: todo.id.map(|id| id.to_hex()),
            created_at: todo.created_at,
            updated_at: todo.updated_at,
        })
    }
}

pub struct Query;

#[async_graphql::Object]
impl Query {
    async fn todos(&self,
        ctx: &Context<'_>,
    ) -> FieldResult<Vec<Todoql>> {
        // this is to ensure that the db connection is always reused
        let store = ctx.data::<Store>()?;
        let todos = store.db.get_all_todos().await?;
        let todos = todos
            .into_iter()
            .map(|todo| Todoql {
                name: todo.name,
                id: todo.id.map(|id| id.to_hex()),
                created_at: todo.created_at,
                updated_at: todo.updated_at,
            })
            .collect();
        Ok(todos)
    }
}

#[tokio::main]
async fn main() {
    // make mongo connection global
    let store = Store::new().await;
    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(store)
        .finish();

    println!("Playground: http://localhost:5011");

    let graphql_post = async_graphql_warp::graphql(schema).and_then(
        |(schema, request): (
            Schema<Query, Mutation, EmptySubscription>,
            async_graphql::Request,
        )| async move {
            Ok::<_, Infallible>(GraphQLResponse::from(schema.execute(request).await))
        },
    );

    let graphql_playground = warp::path::end().and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/")))
    });

    let routes = graphql_playground
        .or(graphql_post)
        .recover(|err: Rejection| async move {
            if let Some(GraphQLBadRequest(err)) = err.find() {
                return Ok::<_, Infallible>(warp::reply::with_status(
                    err.to_string(),
                    StatusCode::BAD_REQUEST,
                ));
            }

            Ok(warp::reply::with_status(
                "INTERNAL_SERVER_ERROR".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        });

    warp::serve(routes).run(([0, 0, 0, 0], 5011)).await;
}
