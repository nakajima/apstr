# apstr

keep ur apps in testflight, monitor status of builds. the usual.

### compose.yml

```yaml
services:
  apstr:
    ports:
      - "8345:8345"
    container_name: apstr
    restart: always
    environment:
      - DATABASE_PATH=/data/apstr.db
      - ASC_KEY_ID=${ASC_KEY_ID}
      - ASC_ISSUER_ID=${ASC_ISSUER_ID}
      - ASC_PRIVATE_KEY_PATH=${ASC_PRIVATE_KEY_PATH}
    volumes:
      - ./apstr/data:/data
    build:
      context: .
      no_cache: true
      dockerfile_inline: |
        FROM ubuntu:latest
        RUN apt-get update
        RUN apt-get install curl libsqlite3-dev -y
        RUN curl -fsSL https://github.com/nakajima/apstr/releases/latest/download/install.sh | bash
    command: ["/usr/local/bin/apstr"]
```