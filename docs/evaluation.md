# かな漢字変換の品質評価

micro benchmarkは速度を、評価suiteは変換品質を測る。AJIMEE-Benchは難しい漢字誤変換へ意図的に偏ったデータなので、単独の総合品質指標にはせず、must-pass testや将来のbalanced corpusと併用する。

## AJIMEE-Bench

実行方法:

```sh
just evaluate-ajimee
just evaluate-ajimee --context none
just evaluate-ajimee --context present --json
```

直接実行する場合:

```sh
scripts/evaluate-ajimee.sh --top-k 10 --context all
```

初回だけ評価データを`target/evaluation`へ取得する。取得元はAJIMEE-Benchのcommit `401666cd56d1a570c2021798b64b6da4396bfd45`に固定し、SHA-256を検証する。評価データを製品bundleへ含めたり、通常のbuildやtestでネットワークへ接続したりしない。

出力する指標:

- `acc@1`: 第1候補がいずれかの許容解と完全一致した割合
- `acc@k`: 上位k候補に許容解が含まれる割合
- `mrr@k`: 最初の正解候補の逆順位の平均
- `mincer@1`: 第1候補と最も近い許容解の文字誤り率
- `mincer@k`: 上位k候補と許容解の組み合わせで最小の文字誤り率
- latency `p50/p95/p99/max`: 辞書初期化を除いた候補生成時間

`--context none`は左文脈なし100件、`--context present`は左文脈あり100件、`--context all`は全200件を評価する。現在の変換器は左文脈を順位付けに使わないため、レポートの`context_used_by_engine`は`false`になる。文脈モデルを導入した場合、この区分を維持して効果を比較する。

### 2026-07-20 baseline

N-bestを10件、ユーザー履歴なし、追加辞書なしで測定した。辞書の初期化時間はlatencyから除外している。

| subset | items | acc@1 | acc@10 | MRR@10 | MinCER@1 | MinCER@10 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 左文脈なし | 100 | 0.350 | 0.580 | 0.418 | 0.100 | 0.059 |
| 左文脈あり（文脈未使用） | 100 | 0.320 | 0.580 | 0.405 | 0.135 | 0.068 |
| 全体 | 200 | 0.335 | 0.580 | 0.412 | 0.117 | 0.064 |

全体の候補生成latencyはp50 6.76 ms、p95 27.03 ms、p99 34.73 ms、最大44.63 msだった。AJIMEEは入力長が最大117文字の難例を含む。通常候補表示の20 ms予算はp95で超えているため、品質改善と並行して長文N-bestの継続的な最適化が必要である。

この初回計測で、未変換のひらがな候補へ固定コストを与えると長文ほど第1候補へ上がる問題が見つかった。未変換候補を変換済み候補より後ろへ固定した結果、左文脈なしの`acc@1`は0.140から0.350へ改善した。

## 元の日本語Wikipedia入力誤りデータセット

AJIMEE-Benchは、日本語Wikipedia入力誤りデータセットv2のtestデータから漢字誤変換200件を抽出し、かな入力、変換範囲、複数の許容解を人手確認した評価用データである。

元データは約70万文対を含み、誤字、脱字、衍字、転字、漢字誤変換など複数の問題が混在する。そのままかな漢字変換評価へ使うのではなく、将来の統計モデルでは次のように扱う。

1. 学習には元データの`train`分割だけを使用する。
2. `kanji-conversion`を抽出し、読みと変換対象範囲を別途生成・検証する。
3. AJIMEE-Benchは元データのtest由来なので、学習や調整には使用しない。
4. must-pass、AJIMEE、balanced corpusの結果を別々に報告する。

## ライセンス

AJIMEE-Benchの評価データと元の日本語Wikipedia入力誤りデータセットはCC BY-SA 3.0。AJIMEE-Benchの`utils.py`と`test_utils.py`はCC0 1.0。評価データはダウンロードキャッシュとして分離し、利用時は各配布元のライセンスと帰属表示に従う。
