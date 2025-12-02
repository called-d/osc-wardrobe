# OscWardrobe

## TODO

* [ ] Lua のスレッドでエラー起きたとき
* [ ] Lua のスレッド watchdog
* [ ] Unity側 Alias エディタ
* [x] auto updater / アップデート内容配信用サーバー（GitHub releases）
* [ ] API 定義

## Dev

```shell
PS> npm run tauri dev -- -- -- -- --overwrite-all-lua
```

```shell
PS> $Env:RUST_BACKTRACE="full"; npm run tauri dev
```

## Build