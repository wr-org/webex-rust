cargo watch -x doc &
cd target/docs && python3 -m http.server
