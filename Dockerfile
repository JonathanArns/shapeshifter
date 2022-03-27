FROM rustlang/rust:nightly-slim

COPY . .
RUN ["cargo", "build", "--release"]

EXPOSE 8080

ENTRYPOINT ["./shapeshifter"]
