# Rust ChatApp

## About

This is a chat application build with Tokio server and frontend build as a TUI app. Connection is established using web socket protocol and server also connects to dabatabse for persistence. Purpose of the project was to get more familiar with standard Rust mechanisms and ecosystem to comfortably use to write efficient programs.

## Deploy (Brower-PTY-client)

App can be run locally normaly in a terminal, but following mechanism was implemented to render i in browser:

Server is deployed in this link: [link text](https://rust-chat-app-uz29.onrender.com/). The client is written as TUI app, meaning it cannot be simply loaded into a browser, TUIs are made to communicate through a PTY channel with a terminal emulator on the other end, a limitation that I solved in the following way: the server sends XTerm.js app which renders UI in the borwser, interprets input and sends it back to server via web socket connection. Server then spawnes a sub process, on which the TUI client is executed and actual PTY channel is created between the server and client app. The app receives the data via PTY channel and sends a response back, again through PTY channel and further through web socket protocol back to the user. Main reason for such shenanigans is that something in the browser needs to translate keyboard interaction as valid ANSI escape sequences, since this is how TUIs are build to communicate with user. This was fun to implement, but is ofcourse far from usable since running clients for every user on backend is expensive. (Deployed version is also free deploy with limited resources, so please don't crash my server, thank you).

### Run locally

either docker with compose or Rust local setup. (Just run docker compose up )

#### ENV

create .env file and pass the following variables:

- SERVER_HOST (localhost)
- SERVER_PORT
- DB_URL (already set with docker, but needed for local setup)

### With Docker

- docker compose up (starts only server)
- cargo run -p client --release

### Locally:

- cargo run -p server --release
- cargo run -p client --release

### Locally:

## Architecture

![Architecture](./assets/server_diagram.png)

![Architecture](./assets/client_diagram.png)

## TODO

- peer to peer communication, where each client is also a server and communicates with other clients through DNS lookup without central server.
