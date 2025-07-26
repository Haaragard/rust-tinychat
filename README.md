# TinyChat

---

## Build & Run
```bash
cargo build
```

```bash
cargo run
```

## TODOs

- [x] HTTP Server
- [x] Can Upgrade connection from HTTP to WebSocket on `GET /messages`
- [x] WebSocket Push - saving messages
- [x] WebSocket Pull - receive new messages
- [ ] Tests

## References

- [\[tiny_http\]](https://github.com/tiny-http/tiny-http/tree/master)
- [\[tiny_http - examples\]](https://github.com/tiny-http/tiny-http/tree/master/examples)
- [\[tiny_http - examples - WebSocket\]](https://github.com/tiny-http/tiny-http/blob/master/examples/websockets.rs)
