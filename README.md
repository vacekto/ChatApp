# Rust ChatApp

## About
Project is a chat application using asynchronous framework Tokio on server written using and frontend, which is build as simple TUI app. Connection is established using raw TCP connection and server also connects to dabatabse for persistence. Purpuse of the project was to get more familiar with standard Rust ecosystem and mechanisms to comfortably use it to write effitient programs. Deploying such personal projects for free proves to be little problematic, works only locally as of yet. 


## How to start
### ENV variables

- SERVER_PORT (optional, default 11111)
- DB_URL (optional, default docker sets DB image by itself, default locally is set remote database using MongoDB Atlas)

### Requirements
either Docker or Rust local dev setup

### With Docker

- docker compose up  (start server)
- cargo run --bin client --release

### Locally:

- cargo run --bin server --release
- cargo run --bin client --release

### Locally:

## TLC

TCP connection is encrypted by self signed certificate saved in the repo along with the private key. This is obviously not secure and inteded only as personal project practice.


## Architecture

![Architecture](./assets/server_diagram.png)

![Architecture](./assets/client_diagram.png)

## TODO
- peer to peer communication, where each client is also a server and communicates with other clients through DNS lookup without central server.
