**To run the project**

1. Install Rust
2. Install Postgres via Docker
3. Clone the repo
4. Run `cargo run`

**Run Postgres in Docker**

docker run -d \
  --name postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=intents \
  -p 5432:5432 \
  -v $(pwd)/init.sql:/docker-entrypoint-initdb.d/init.sql \
  -v pgdata:/var/lib/postgresql/data \
  postgres:16

  *Connection string for Postgres* 
 `postgres://postgres:postgres@localhost:5432/intents`

 API server: http://localhost:3000/

 The onchain vault contract is a WIP.
