# aopkg

aopkg is a package registry for Anarchy Online chatbots. It provides a web interface and API to interact with and an easy package layout.

## Package layout

You can read about it in the [PACKAGING.md](PACKAGING.md) file.

## Running

You'll need a working rust compiler and cargo.

```bash
cargo build --release
touch aopkg.db
touch .env
# put config in .env
./target/release/aopkg
```

Or use docker/podman:

```bash
podman build -t aopkg:latest .
touch aopkg.db
touch .env
# put config in .env
podman run --rm -it -p 7575:7575 --env-file .env -v $(pwd)/aopkg.db:/aopkg.db:Z -v $(pwd)/data:/data:Z aopkg:latest
```

## Configuration

`.env` should look like this:

```
DATABASE_URL=sqlite:aopkg.db
COOKIE_SECRET=some_32_character_string
CLIENT_ID=github_client_id
CLIENT_SECRET=github_client_secret
```
