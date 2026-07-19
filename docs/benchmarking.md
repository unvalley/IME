# ベンチマーク方針

## 目的

micro benchmarkは、変更前後で同じhot pathを比較するために使う。異なる端末の絶対値を製品性能として比較しない。必ずrelease profileで計測し、一度に一つの変数だけを変更する。

現在は外部benchmark crateを使わず、`std::hint::black_box`と`Instant`による小さなharnessを使う。辞書規模が大きくなり統計的な比較が必要になった時点でCriterionまたはDivanを再評価する。

## 計測対象

| benchmark | 内容 | 初期性能予算 |
| --- | --- | ---: |
| `romaji/nihongo` | `nihongo`全体のincremental変換とflush | 1キー平均 p95 5 ms未満の十分内側 |
| `converter/exact_candidates` | 完全一致候補の生成とsort | 20 ms未満 |
| `converter/segmented_phrase` | `わたしはにほん`のラティス探索 | 20 ms未満 |
| `engine/nihon_conversion` | engine生成から入力、変換、確定まで | 参考値。cold startを分離予定 |

現状の辞書は数語なので、上限を満たすこと自体に意味はない。辞書10万、25万、50万entryでのサイズ・RSS・p50/p95/p99を測って初めて製品判断に使う。

## 初回baseline

2026-07-19、Apple M3、arm64、macOS 26.5.1で採取。表示値は1操作あたりの単純平均で、p95ではない。

| benchmark | 結果 |
| --- | ---: |
| `romaji/nihongo` | 7,847 ns/op |
| `converter/exact_candidates` | 1,573 ns/op |
| `converter/segmented_phrase` | 2,237 ns/op |
| `engine/nihon_conversion` | 18,174 ns/op |

反復回数は順に50,000、25,000、25,000、10,000。端末状態による揺れがあるため、最適化判断では同じprocess、同じ反復回数で複数回測る。

## 実行方法

```sh
cargo bench -p ime-romaji --bench romaji
cargo bench -p ime-converter --bench converter
cargo bench -p ime-core --bench engine
```

短いsmoke run:

```sh
IME_BENCH_ITERATIONS=10000 cargo bench -p ime-core --bench engine
```

## 今後追加する計測

- allocation countと割り当てbyte数
- compiled dictionaryのファイルサイズ
- mmap直後とwarm後のRSS
- cold/warm prefix lookup
- 候補数10/100/1000件
- 10/50/100文字の入力
- user dictionaryあり/なし
- Swift → C ABI → Rust → Swiftのend-to-end latency
- TextEditでのkey down → marked text反映時間

性能を理由に`unsafe`、特殊なhasher、arena、small-string最適化を導入する場合は、先にこのbenchmarkでbottleneckを示す。

