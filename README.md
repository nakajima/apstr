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

### hook scripts

Each app can define a hook script from its show page. apstr runs it with:

```sh
/bin/sh -c "$hook_script"
```

Hooks run asynchronously, time out after 30 seconds, and do not fail the sync or build that triggered them. Recent runs capture exit code, stdout, stderr, and errors.

Events:

- `build_started`
- `build_auto_started`
- `build_completed`
- `testflight_expired`
- `testflight_expiring`

Common environment variables:

- `APSTR_EVENT`
- `APSTR_EVENT_LABEL`
- `APSTR_APP_ID`
- `APSTR_APP_NAME`
- `APSTR_BUNDLE_IDENTIFIER`
- `APSTR_ASC_APP_ID`
- `APSTR_APP_URL` when `APSTR_BASE_URL` is set
- `APSTR_ASC_APP_URL`
- `APSTR_TESTFLIGHT_URL`

Build events may include:

- `APSTR_BUILD_ID`
- `APSTR_BUILD_NUMBER`
- `APSTR_BUILD_STATUS`
- `APSTR_ASC_BUILD_URL`
- `APSTR_WORKFLOW_ID`
- `APSTR_WORKFLOW_NAME`

TestFlight events may include:

- `APSTR_TESTFLIGHT_VERSION`
- `APSTR_TESTFLIGHT_EXPIRATION_STATUS`
- `APSTR_TESTFLIGHT_EXPIRATION_DATE`

Example:

```sh
case "$APSTR_EVENT" in
  build_completed)
    echo "$APSTR_APP_NAME: build $APSTR_BUILD_NUMBER completed with $APSTR_BUILD_STATUS"
    ;;
esac
```
