# Rust ChatApp

## About
Project is a chat application with server written using asynchronous Tokio framework and frontend app as a TUI, in which concrrency is handled with system threads provided by standard library api. Connection is established using raw TCP connection and server also connects to dabatabse for persistence. 
Purpuse of the project was to get more familiar with standard Rust ecosystem and mechanisms to comfortably use it to write effitient programs. Deploying such personal projects for free proves to be little problematic, works only locally as of yet. 


## How to start
with docker:

- set SERVER_PORT env variable (is enough to create .env and declare variable there)
- docker compose up  (start server)
- cargo run --bin client --release

without docker: 

- default server port is 11111 in case of absent SERVER_PORT env var. Set SERVER_PORT or change in clienr.rs and server.rs
- cargo run --bin server --release
- cargo run --bin client --release

Docker pulls Mongo image to connect to, but remote database connection is setup in case of running locally. To change set DB_URL env variable.

## Architecture

- ### Server
Server immediately spawns two asynchornous tasks called "manager_task" and "persistence_task" with separate responsibilities. Once TCP connection is established and user logs in, "client_task" struct gets initialized and started. This holds ownership of the only TCP connection instance for client and messages are being framed to eliminate 
message data interleaving. Manager_task holds bidirectional communication with every client_task via Rust channels and keeps list of currently connected clients in a hashmap, whereas persistence_task is responsible for fetching DB data. 

- ### Client_task
Struct representing connected user, basically backbone of server logic along with manger_task. Fethes persisted user data during initialization and holds channel transmitters in memory for every room the client is in and every other client currently in communication with.
Listening for each room is blocking operation, so new tokio task called "communication_task" is created dynamically for each room upon "room join" or "init" events where. From this communication_task the channel receiver delegates data to the listening client_task and data get furher sent to user through TCP stream instance.

- ### Manager_task
Manager_task is responsible for establishing channel communication between separate client_tasks, forming simple implementation of so called "Pub/Sub" system. Upon initialization of client_task, manager task checks for every room the user is present in for any other 
already online users. If there are any, manager_tasks sends request for the room transmitter to one of those clients along with oneshot channel for tranferring the wanted room transmitter. If no user is online, new trasnimmter gets created and sent back using the very same oneshot channel.
Direct channels between clients gets established dynamically in similar manner. This solves necesity to send every client_task every message only for them to further check whether the message is actually meant for corresponding user. This way, raw serialized bytes can be transmitted through channels directly to targeted TCP connection once the communication_tasks get created.

- ### Persistence_task
Listens for events from clients for DB fetching and responds with oneshot transmitter from the client_task. Db used is MongoDB with official Rust MongoDB driver. ORM would be more comfortable for larger projects in my opinion.

- ### Client
Client is able to send text messages, converted images to ASCII art and tranfer files. Files are beeing saved to "files" folder in the app folder.

## TODO
- TLS support. TLS for server is straightforward to add, client however is more troublesome. TCP stream instance is split to write and read halfs, which cannot be done for blocking encrypted TCP connection. Either small client rewrite is needed (at which point client would need to periodically check for any received messages so it could write to server in the meantime, which is not perfect solution in case of streaming files), complete Client rewrite to Tokio, since async version is capable of splitting TCP stream for some reason, or implementing custom split for the enncrypted TCP instance.
- peer to peer communication, where each client is also a server and communicates with other clients through DNS lookup without central server.
