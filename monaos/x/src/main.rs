use serde::{Deserialize, Serialize};
use std::io::{self, Read};
use std::collections::HashMap;
use std::sync::Mutex;
use karics::{HttpServer, HttpService, Request, Response};

// Define the User struct
#[derive(Serialize, Deserialize, Clone)]
struct User {
    id: u32,
    name: String,
    email: String,
}
// Implement the User
#[derive(Clone)]
struct ApiService {
    users: std::sync::Arc<Mutex<HashMap<u32, User>>>
}

// Implement the ApiService
impl ApiService {
    fn new() -> Self {
        ApiService {
            users: std::sync::Arc::new(Mutex::new(HashMap::new()))
        }
    }
}

impl HttpService for ApiService {
    fn call(&mut self, req: Request, res: &mut Response) -> io::Result<()> {
        match (req.method(), req.path()) {

            // Get all users
            ("GET", "/users") => {
                let users = self.users.lock().unwrap();
                let users_vec: Vec<User> = users.values().cloned().collect();
                let json = serde_json::to_string(&users_vec)?;
                res.body(Box::leak(json.into_boxed_str()));
            }
            // Get user by ID
            ("GET", path) if path.starts_with("/users/") => {
                let id = path.trim_start_matches("/users/");
                if let Ok(user_id) = id.parse::<u32>() {
                    let users = self.users.lock().unwrap();
                    match users.get(&user_id) {
                        Some(user) => {
                            let json = serde_json::to_string(&user)?;
                            res.body(Box::leak(json.into_boxed_str()));
                        }
                        None => {
                            res.status_code(404, "Not Found");
                            res.body("User not found");
                        }
                    }
                } else {
                    res.status_code(400, "Bad Request"); 
                    res.body("Invalid user ID");
                }
            }
            
            // Create user
            ("POST", "/users") => {
                let mut body = Vec::new();
                req.body().read_to_end(&mut body)?;
                match serde_json::from_slice::<User>(&body) {
                    Ok(user) => {
                        let mut users = self.users.lock().unwrap();
                        users.insert(user.id, user.clone());
                        let json = serde_json::to_string(&user)?;
                        res.status_code(201, "Created");
                        res.body(Box::leak(json.into_boxed_str()));
                    }
                    Err(_) => {
                        res.status_code(400, "Bad Request");
                        res.body("Invalid user data");
                    }
                }
            }
            
            // Update user
            ("PUT", path) if path.starts_with("/users/") => {
                let id = path.trim_start_matches("/users/");
                if let Ok(user_id) = id.parse::<u32>() {
                    let mut body = Vec::new();
                    req.body().read_to_end(&mut body)?;
                    match serde_json::from_slice::<User>(&body) {
                        Ok(updated_user) => {
                            let mut users = self.users.lock().unwrap();
                            if users.contains_key(&user_id) {
                                users.insert(user_id, updated_user);
                                res.status_code(200, "OK");
                                res.body("User updated");
                            } else {
                                res.status_code(404, "Not Found");
                                res.body("User not found");
                            }
                        }
                        Err(_) => {
                            res.status_code(400, "Bad Request");
                            res.body("Invalid user data");
                        }
                    }
                } else {
                    res.status_code(400, "Bad Request");
                    res.body("Invalid user ID");
                }
            }

            // Delete user
            ("DELETE", path) if path.starts_with("/users/") => {
                let id = path.trim_start_matches("/users/");
                if let Ok(user_id) = id.parse::<u32>() {
                    let mut users = self.users.lock().unwrap();
                    if users.remove(&user_id).is_some() {
                        res.status_code(204, "No Content");
                        res.body("");
                    } else {
                        res.status_code(404, "Not Found");
                        res.body("User not found");
                    }
                } else {
                    res.status_code(400, "Bad Request");
                    res.body("Invalid user ID");
                }
            }

            _ => {
                res.status_code(404, "Not Found");
                res.body("Not Found");
            }
        }
        Ok(())
    }
}

// Main function to start the server on http://
fn main() {
    println!("Server starting on http://0.0.0.0:8080");
    let server = HttpServer(ApiService::new()).start("0.0.0.0:8080").unwrap();
    server.join().unwrap();
}
