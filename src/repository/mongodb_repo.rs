use std::env;
    extern crate dotenv;
    use async_graphql::futures_util::TryStreamExt;
    use dotenv::dotenv;

    use mongodb::{
        bson::{extjson::de::Error, doc},
        Client, Collection,
    };
    use crate::models::todo_model::Todo;

    pub struct MongoRepo {
        col: Collection<Todo>,
    }

    impl MongoRepo {
        pub async fn init() -> Self {
            dotenv().ok();
            let uri = match env::var("MONGOURI") {
                Ok(v) => v.to_string(),
                Err(_) => format!("Error loading env variable"),
            };
            let client = Client::with_uri_str(uri).await.unwrap();
            let db = client.database("rustmongo");
            let col: Collection<Todo> = db.collection("todo");
            MongoRepo { col }
        }

        pub async fn create_todo(&self, new_todo: Todo) -> Result<Todo, Error> {
            let new_doc = Todo {
                id: None,
                name: new_todo.name,
                created_at: new_todo.created_at,
                updated_at: new_todo.updated_at,
            };

            let todo = self
                .col
                .insert_one(new_doc, None)
                .await
                .ok()
                .expect("Error creating todo");

            // get the id of the inserted document
            let id = todo.inserted_id.as_object_id().unwrap();
            // find the document by id
            let todo_result = self
                .col
                .find_one(Some(doc! {"_id": id}), None)
                .await
                .ok()
                .expect("Error finding todo");
            
            Ok(todo_result.unwrap())
        }
        pub async fn get_all_todos(&self) -> Result<Vec<Todo>, Error> {
            let mut cursors = self
                .col
                .find(None, None)
                .await
                .ok()
                .expect("Error getting list of todos");
            let mut todos: Vec<Todo> = Vec::new();
            while let Some(todo) = cursors
                .try_next()
                .await
                .ok()
                .expect("Error mapping through cursor")
            {
                todos.push(todo)
            }
            Ok(todos)
            }
    }