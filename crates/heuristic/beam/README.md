# beam

差分更新型ビームサーチ。状態の更新・巻き戻しや候補生成を `Logic` で実装する。
探索木のバックエンドは、1手1ターンなら `PostorderTour`、世代スキップが必要なら
`LinkedTree`、状態をコピーする照合用なら `NaiveCopy` を使う。
詳しい契約と使用例は [モジュールのドキュメント](src/beam/mod.rs) を参照。

```sh
cargo doc -p beam --no-deps
cargo test -p beam
cargo bench -p beam --bench tree
```

フェーズごとの性能計測には `profile` feature を使う。通常のビームサーチは標準ライブラリだけで動く。
このクレートは hlib-rs の `ff6b3e226269` を元にしている。
