# 軽量日本語IMEのためのかな漢字変換・入力研究レビュー

- 調査日: 2026-07-19
- 対象: デスクトップ向け日本語IME、かな漢字変換、辞書・言語モデル圧縮、個人適応、入力誤り訂正、低遅延、評価方法
- 現在の実装対象: macOS
- 将来の設計対象: Windows、Linux

## 1. このレビューの目的

この文書は、単に論文を列挙するのではなく、次の製品判断に答えるための文献レビューである。

1. 小さな辞書と言語モデルで、通常のかな漢字変換をどこまで実現できるか。
2. 精度、配布サイズ、常駐メモリ、入力遅延のどれを優先すべきか。
3. ユーザー学習や未知語追加が、既存の変換を壊さないために何が必要か。
4. ニューラル変換は初期版へ入れるべきか。
5. 「使えるIME」を何で評価するべきか。

調査対象はACL Anthology、言語処理学会、J-STAGE、CiNii、ACM、公式コーパス・実装資料を中心にした。かな漢字変換そのものを扱う研究を優先し、一般的な自然言語処理やスマートフォンのキーボード配置研究は、直接設計判断へつながるものだけを含めた。

## 2. 結論

文献から支持される初期方針は次の通りである。

1. **第一候補は、読み辞書 + クラスbigram + 読みモデル + Viterbiである。**
2. **辞書の語数を減らすだけではなく、LOUDS、文字列共有、品詞IDの可変長符号化、カタカナ表記の生成によって格納形式を圧縮する。**
3. **言語モデルは全面的な高次n-gramより、クラス化、phrase化、pruning、量子化を順に試す。**
4. **平均CERや完全一致率だけで出荷しない。重要語のmust-pass回帰、候補順位、学習副作用、訂正操作数、p95遅延を同時に測る。**
5. **ユーザー学習は、選択候補を無条件で昇格させない。文脈、機能語、品詞の交換可能性、上限、減衰、rollbackを持たせる。**
6. **未知語は最初から巨大辞書へ寄せず、生成候補、ユーザー辞書、限定的な追加辞書で扱う。**
7. **誤入力訂正は自動置換ではなく、明示的な候補提示から始める。**
8. **ニューラル変換は初期版へ入れない。将来も統計エンジンのN-bestリスコアラーまたは任意追加モデルとして評価する。**

最も重要な発見は、軽量化と品質が単純な二者択一ではないことである。Mozcの辞書圧縮研究では、1,345,900語を持つ辞書を59.1 MBのプレーンテキストから13.3 MBへ圧縮し、共通接頭辞検索、予測検索、逆引きを維持している。語彙を極端に削る前に、表現形式を改善する余地が大きい。

## 3. 研究の系譜

かな漢字変換研究は、概ね次の流れで発展している。

```text
規則・最長一致
    ↓
確率的変換
読みモデル × n-gram言語モデル × Viterbi
    ↓
クラス化・複合語化・未知語・ドメイン適応
    ↓
識別モデルと副作用を考慮した個人適応
    ↓
辞書・言語モデルの圧縮と製品評価
    ↓
ニューラル言語モデル、逐次変換、低遅延decoding
```

ニューラル研究が従来方式を不要にしたわけではない。2019年のリアルタイムニューラルIMEも、辞書から作るword latticeとViterbi decoderを維持した上でLSTMを言語モデルとして使っている。Zenzaiも実製品では辞書ベース変換と組み合わせられている。軽量IMEでは、統計エンジンが基盤であり続ける。

## 4. 主要研究の比較

| 年 | 研究・資料 | 主題 | 重要な結果 | 今回の判断 |
| ---: | --- | --- | --- | --- |
| 1997 | 山本ほか「かな漢字変換における誤入力の訂正」 | 置換・挿入・脱落誤り | 自動訂正は再現率44%、誤り率3% | 自動修正は初期不採用。候補提示を優先 |
| 1998/2000 | Daciuk et al. | minimal acyclic automata | 辞書を小さく高速に保持する構築法 | FST/DAFSAをLOUDSとの比較候補にする |
| 1998 | Stolcke | n-gram pruning | 元モデルの26%へ縮小して認識誤りを増やさない例 | 頻度だけでなくentropy基準でpruneする |
| 1999 | 森ほか「確率的モデルによる仮名漢字変換」 | 確率的かな漢字変換 | 読みモデルと言語モデルによる変換を確立 | 初期エンジンの基礎 |
| 2003 | Soukoreff & MacKenzie | 文字入力評価 | MSD、KSPCと訂正済み/未訂正誤りを統合 | 確定結果だけでなく訂正操作も計測 |
| 2005 | Suzuki & Gao | LM適応 | 識別的適応がCERと副作用の両面で優位 | 学習のside effectを独立指標にする |
| 2006 | 森「単語リストと生コーパスによる言語モデル適応」 | ドメイン適応 | 対象語周辺へ注釈を集中すると少ない作業量で改善 | 専門辞書の評価方法に採用 |
| 2010 | 笹田・森・河原 | 未知語 | 類似テキストと音声から読み・文脈を獲得 | 初期実装には重いが未知語評価の基準 |
| 2011 | 工藤ほか「統計的かな漢字変換システム Mozc」 | 製品IME全体 | 学習副作用、文節、N-best、must-pass回帰を整理 | 最重要の製品設計資料 |
| 2011 | Tokunaga et al. | structured SVM | 生成モデルより1〜4%改善 | 将来のreranker候補。初期モデルにはしない |
| 2011 | Kudo et al. | 辞書・LM圧縮 | 59.1→13.3 MB、LM 17.4→2.9 MB | 軽量化の中核 |
| 2011 | Kasahara et al. | romaji-kana誤り訂正 | ローマ字入力誤りを変換前に訂正 | 外国語・typo支援の将来候補 |
| 2011 | Heafield | n-gram格納 | probingとbit-packed trieの速度・容量比較 | 自作LM格納形式の比較基準 |
| 2012 | Maeta & Mori | phrase class n-gram | 小さいbigramでtrigram同等以上のF値 | phrase/class化を実験候補にする |
| 2014 | Maekawa et al. | BCCWJ | 約1億語のbalanced corpus | 評価候補。製品modelへの利用条件は別途確認 |
| 2019 | Yao et al. | neural IMEの低遅延化 | incremental vocabulary selectionで1キー3 ms | neural導入時の基準。初期不採用 |
| 2021 | 田中ほか | 日本語入力誤りデータ | Wikipedia履歴から約70万文対 | typo・誤変換評価データ候補 |
| 2023 | Schmid et al. | 入力遅延のHCI評価 | 20 msと200 msで訂正と主観負荷に差 | p95遅延を出荷指標にする |
| 2024 | Sarhangzadeh & Watanabe | 逐次neural KKC | word boundaryを使う低遅延policy | ライブ変換を行う場合だけ再評価 |
| 2025 | ensan「Zenzai」 | local neural KKC | 量子化後も19.9〜237.2 MB | 初期サイズ予算外。任意追加のみ |
| 2025 | AJIMEE-Bench | 難しい誤変換200件 | 左文脈あり/なし各100件、複数許容解 | 回帰suiteの一部。単独の総合指標にしない |

## 5. 確率的かな漢字変換

### 5.1 基本モデル

[確率的モデルによる仮名漢字変換](https://cir.nii.ac.jp/crid/1050845762815505920)は、入力かな列に対して最も確率の高いかな漢字混じり文を選ぶ方式を示した。後続研究では、概ね次の形で定式化される。

```text
best output = argmax P(output) × P(reading | output)
```

- `P(output)`: 単語列またはクラス列の言語モデル
- `P(reading | output)`: 表記がその読みで入力される確率
- decoding: 読みの各位置から候補nodeを作るDAG上のViterbi探索

[統計的かな漢字変換システム Mozc](https://www.anlp.jp/proceedings/annual_meeting/2011/pdf_dir/C4-3.pdf)は、クラスbigram `P(class_i | class_i-1)`、クラス内の単語確率、単語と読みの確率を組み合わせている。品詞、活用形、助詞・助動詞、頻出語を使って約3,000クラスを構成した。

今回の初期モデルは、これをさらに小さくした次の構成から始める。

```text
entry cost = reading cost + word-in-class cost
transition cost = class bigram cost
path cost = sum(entry cost + transition cost)
```

### 5.2 なぜ生成モデルから始めるか

[Discriminative Method for Japanese Kana-Kanji Input Method](https://aclanthology.org/W11-3502/)ではstructured SVMが生成モデルを1〜4%上回った。一方で、Mozcの設計論文は製品上の理由として次を挙げている。

- 大規模データに対する学習のscalability
- N-best候補全体の順位を説明しやすいこと
- 頻度・確率としてparameterを解釈できること
- 読みと表記を逆に引く再変換へ展開しやすいこと

軽量IMEの初期段階では、変換失敗を辞書、単語cost、接続costへ分解して直せることが重要である。識別モデルは、統計ベースラインと回帰データが揃った後のN-best rerankerとして比較する。

### 5.3 classとphrase

[Statistical Input Method based on a Phrase Class n-gram Model](https://aclanthology.org/W12-4801/)はBCCWJ-Coreを用い、次の結果を報告した。

| モデル | F値 | 語彙数 | 非ゼロ頻度 |
| --- | ---: | ---: | ---: |
| word-pronunciation bigram | 90.09 | 22,801 | 264,336 |
| class bigram | 90.03 | 4,245 | 141,482 |
| phrase bigram | 90.39 | 25,056 | 339,574 |
| phrase class bigram | 90.41 | 5,550 | 206,978 |
| word-pronunciation trigram | 90.21 | 22,801 | 645,996 |

異なるデータや実装でそのまま再現される数値ではないが、高次n-gramを全面採用する前にclass化と頻出phrase化を試す根拠になる。

初期実験は次の順序にする。

1. word unigram
2. class bigram
3. 頻出機能語のlexicalization
4. 頻出複合語・定型句のphrase化
5. entropy-pruned word bigramとの比較
6. 必要な場合だけtrigram reranking

## 6. 辞書と言語モデルの軽量化

### 6.1 IME専用の圧縮研究

[Efficient dictionary and language model compression for input method editors](https://aclanthology.org/W11-3503/)は、今回の軽量化方針に最も直接的な研究である。

辞書は読みtrie、表記trie、token配列へ分離され、LOUDSによってpointerを排除する。さらに次を組み合わせる。

- 共有可能なprefix/suffixをまとめる文字列圧縮
- 頻出する品詞IDへ短い符号を割り当てるtoken圧縮
- ひらがなとカタカナの一対一対応を利用し、カタカナ表記をbitで生成

実験結果:

| 辞書表現 | サイズ | 1語あたり |
| --- | ---: | ---: |
| plain text | 59.1 MB | 46.0 byte |
| double array | 80.8 MB | 63.0 byte |
| LOUDS + token | 20.5 MB | 16.0 byte |
| LOUDS + 全heuristic | 13.3 MB | 10.4 byte |

対象辞書は1,345,900語で、共通接頭辞、予測、逆引きをサポートした。double-arrayがこの実験ではplain textより大きい点も重要であり、データ構造名だけで選ばず、実データで測る必要がある。

約3,000クラスの遷移表では86%がゼロで、2次元配列17.4 MBに対しsuccinct treeは2.9 MBだった。小さな512-entry cacheで変換時間は1文あたり0.0158秒から0.0102秒へ改善している。

### 6.2 一般的な辞書構造から得る選択肢

[Incremental Construction of Minimal Acyclic Finite-State Automata](https://aclanthology.org/J00-1002/)は、語の集合をminimal acyclic automatonとして逐次構築する方法を示す。共通suffixも共有できるため、通常のtrieより小さくなる可能性がある。

[KenLM](https://aclanthology.org/W11-2123/)は、高速なlinear probingと省メモリなbit-packed trieを比較し、確率値の量子化やmemory localityを含めて設計している。IME専用ではないが、n-gram lookupの速度・容量trade-offを測る基準になる。

実装では最初からLOUDSへ固定せず、同じ辞書compilerから少なくとも次を生成して比較する。

- sorted flat array + binary search
- compact trie/LOUDS
- minimal FST/DAFSA

比較項目はファイルサイズ、mmap後RSS、cold lookup、warm lookup、prefix列挙、N-best変換全体である。

### 6.3 pruningと量子化

[Entropy-based Pruning of Backoff Language Models](https://arxiv.org/abs/cs/0006025)は、n-gramを削ったときのrelative entropy増分を基準にpruneする。実験ではproduction-quality Hub4モデルを元の26%へ縮小し、認識誤りを増やさなかった。

今回のモデル生成では、次を別々に評価する。

1. occurrence countによる単純cutoff
2. held-out conversionへの寄与
3. entropy-based pruning
4. costの16-bit、12-bit、8-bit量子化

最終的な目的関数はperplexity最小化ではなく、変換回帰を壊さずサイズとRSSを下げることである。

## 7. 個人適応と学習副作用

### 7.1 CER改善だけでは足りない

[A Comparative Study on Language Model Adaptation Techniques Using New Evaluation Metrics](https://aclanthology.org/H05-1034/)は、MAP、boosting、averaged perceptron、minimum sample riskを比較し、識別的手法がCER削減と副作用の少なさでlinear interpolationを上回ると報告した。

この研究の重要点は、同じCERでも既存の正解を誤りへ変えた数が異なり得ることである。adaptationの評価には次を分ける必要がある。

- 改善: baselineで誤り、adapted modelで正解
- 副作用: baselineで正解、adapted modelで誤り
- 未変化の正解/誤り

### 7.2 Mozcの交換可能制約

Mozcの論文では、ユーザーが「きょうと」を「京都」ではなく「京と」と確定した場合、その結果を無条件学習すると、次回の「きょう」まで「京」が優先され得る問題を説明している。そこで、内容語の品詞大分類と機能語の表層が一致する場合だけ、一部の学習を許可している。

初期の学習仕様:

- `reading + surface + left class + right class`を小さなdeltaとして保存
- 品詞・機能語境界が大きく変わる選択は強いunigram学習をしない
- 同一の完全なreading/contextでは再現性を優先する
- 頻度に上限を設け、時間減衰させる
- システムモデルへmergeせず、別ファイルでrollback可能にする
- incognito/secure fieldでは読み書きしない

学習なし、学習直後、一定期間経過後の3条件で回帰を取る。

### 7.3 ドメイン適応

[単語リストと生コーパスによる言語モデル適応](https://doi.org/10.5715/jnlp.13.4_33)は、生コーパス全体を完全に人手修正するより、対象語が現れる周辺へ修正作業を集中させる方が、少ない作業量で良いモデルを作れることを示した。

将来、医療・法律・開発用語の追加辞書を作る場合は、語を登録するだけでなく、その語を含む短い文脈の回帰データを同時に用意する。

## 8. 未知語・新語・固有名詞

[自動獲得した未知語の読み・文脈情報による仮名漢字変換](https://doi.org/10.5715/jnlp.17.4_131)は、類似内容のテキストと音声から未知語候補、複数の読み、文脈を獲得し、ニュースデータで変換精度を改善した。

これは未知語に「表記と読み」だけでなく「どの文脈で出すか」が必要であることを示す。初期版では音声認識やWeb miningを搭載せず、次の順序で扱う。

1. 数字、日付、時刻、記号、カタカナをruleで生成
2. ユーザー辞書へ読み・表記・品詞を登録
3. 語ごとに安全なdefault costを設定
4. 追加辞書には対象文脈の回帰suiteを必須にする
5. 語彙追加で既存のmust-pass変換が落ちた場合は配布しない

## 9. 入力誤り訂正

### 9.1 自動訂正は保守的にする

[かな漢字変換における誤入力の訂正](https://cir.nii.ac.jp/crid/1573950401966931328)は、1箇所の置換・挿入・脱落を扱い、選択型訂正で上位6候補までの再現率59%、自動訂正で再現率44%、誤り率3%を報告した。

古い研究で現在のkeyboard/corpusとは条件が異なるが、3%の誤訂正はIMEでは大きい。ユーザーが意図した珍しい固有名詞を「一般的な語」へ勝手に変える危険がある。

初期方針:

- 完全一致する辞書候補がない場合だけtypo候補を生成
- 通常候補と視覚的に区別する
- 自動確定しない
- 編集距離1を基本とし、keyboard隣接やローマ字規則で重み付けする
- typo候補の選択を通常の語彙学習と分離する

### 9.2 ローマ字段階の訂正

[Error Correcting Romaji-kana Conversion](https://aclanthology.org/W11-3506/)は、日本語学習者のローマ字入力誤りを、かな漢字変換前に訂正する方式を扱う。これはかな列の誤り訂正とは別レイヤーである。

ローマ字変換器では、少なくとも次を区別してログなしでテストする。

- 未確定prefix: `n`, `k`, `sh`
- 合法な表記揺れ: `shi/si`, `chi/ti`, `fu/hu`
- 小書き: `xtu/ltu`, `xya/lya`
- 促音、撥音、長音
- typo候補

## 10. ニューラルかな漢字変換

### 10.1 精度向上とモデルサイズ

[Enabling Real-time Neural IME with Incremental Vocabulary Selection](https://aclanthology.org/N19-2001/)は、50,000語のLSTM言語モデルをword latticeと組み合わせた。実験ではtrigramのtop-1が55.60%、LSTM baselineが61.20%だった。一方、pruningなしのn-gramは1 GBを超えるとされ、単純比較はできない。

incremental vocabulary selectionは、latticeに現れる数百語だけをsoftmax対象とし、1キーあたり3 ms、softmax部分でbaseline比84倍の高速化を報告した。この結果はニューラル方式でも低遅延化できることを示すが、モデル本体の配布サイズ、RSS、cold startは別に評価する必要がある。

[ニューラルかな漢字変換システム Zenzai](https://www.anlp.jp/proceedings/annual_meeting/2025/pdf_dir/P1-19.pdf)は文脈を使う高精度な変換を示すが、Q5_K_M量子化後のモデルはxsmall 19.9 MB、small 72.3 MB、medium 237.2 MBである。初期IME全体のダウンロード目標10 MBを、最小モデルだけで超える。

### 10.2 逐次変換

[Alignment-Based Decoding Policy for Low-Latency and Anticipation-Free Neural Japanese IMEs](https://aclanthology.org/2024.findings-acl.479/)は、word boundaryを使ってTransformerをincrementalにdecodeし、品質と遅延のtrade-offを改善する。研究はライブ変換やopt-out conversionを想定している。

今回の初期UIはSpaceで明示的に変換するため、この複雑さは不要である。ライブ変換を検討するときに、次の条件を満たす方式として再評価する。

- 過去prefixのencoder stateを再計算しない
- 中間出力の書き換え回数を測る
- 計算遅延と、入力待ちによる非計算遅延を分ける
- ユーザーが部分候補を修正できるlatticeを維持する

### 10.3 採用境界

ニューラル方式は次の条件を全て満たした場合だけ、任意追加機能にする。

- 既定の統計エンジンだけでIMEが完全に動く
- モデルは本体とは別ダウンロード
- CPUだけでp95候補表示20 ms以内
- 統計N-bestから選ぶrerankerとして部分修正可能
- モデルなし・破損・非対応CPUで安全にfallback
- 入力を外部送信しない

## 11. 評価方法

### 11.1 変換精度

論文ごとに正解定義が異なるため、一つの数値へ集約しない。

| 指標 | 測るもの | 弱点 |
| --- | --- | --- |
| Exact Match / Acc@1 | 文全体が許容解と一致 | 表記揺れへ厳しすぎる |
| CER | 文字単位の編集誤り | 候補選択操作を表さない |
| LCS precision/recall/F | 最長共通部分列 | 語境界や重大な誤りの重みが均一 |
| Acc@k | 正解が上位k候補にあるか | 順位差を粗くしか表さない |
| MRR | 正解候補の順位 | 候補UIの操作量とは完全一致しない |
| segment accuracy | 文節単位の一致 | 文節正解定義がsystem依存 |

一つの語に複数の正しい表記があるため、許容解集合を持つ。ひらがな/漢字の表記差をすべて誤り扱いする厳密評価と、許容表記をまとめる寛容評価を併記する。

### 11.2 実使用の訂正コスト

[Metrics for Text Entry Research](https://doi.org/10.1145/642611.642632)は、最終文字列のminimum string distanceだけでは入力途中で修正した誤りを失うことを指摘し、corrected、uncorrected、total error rateを整理した。

IMEではraw key eventを外部送信せず、ローカルテスト時だけ次を計測する。

- 1文字確定あたりのkey数
- Space/候補移動/文節変更/Backspaceの回数
- 訂正済み誤りと未訂正誤り
- 目標文を完成するまでの時間
- 最初の誤変換から正しい確定までの操作数

### 11.3 平均精度とmust-pass回帰

Mozcの論文では、Web、新聞、検索queryなどの平均評価だけでは実利用の違和感を捉えられず、「昨日」「午後」のような基本語が変換できない問題を防ぐため、絶対に誤ってはいけない例を出荷条件へ追加したと説明している。

評価suiteを次の四層にする。

1. **Core must-pass**: 基本語、助詞、活用、数字、日付
2. **General corpus**: balancedな文章のCER、LCS-F、Acc@k
3. **Difficult cases**: 同音異義、誤変換報告、AJIMEE-Bench
4. **Personalization safety**: 学習で改善した例と壊れた例

平均スコアが上がってもCore must-passが1件落ちたbuildは出荷しない。

### 11.4 AJIMEE-Benchの位置づけ

[AJIMEE-Bench](https://github.com/azooKey/AJIMEE-Bench)は、日本語Wikipedia入力誤りデータセットから漢字変換誤り200件を抽出し、左文脈あり100件、なし100件を人手確認したデータである。複数の許容解を持てる点がよい。

一方で、200件の難例に意図的に偏っており、日常文全体の分布、候補操作、速度、学習副作用は測れない。回帰suiteの「Difficult cases」として採用し、総合ランキングには使わない。

### 11.5 BCCWJ

[Balanced Corpus of Contemporary Written Japanese](https://link.springer.com/article/10.1007/s10579-013-9261-0)は約1億語のbalanced corpusで、多くのかな漢字変換研究が学習・評価に使っている。manual annotationを持つCoreは評価に有用である。

ただし、研究利用できることと、そこから生成した辞書・モデルを製品へ再配布できることは同義ではない。購入・利用条件、原著作物の権利、生成物の配布可否を確認するまで、製品辞書のsourceとは確定しない。

## 12. 遅延と体感品質

[Effects of Text Input Latency on Performance and Task Load](https://doi.org/10.1145/3626705.3627784)は20 msと200 msの入力遅延を比較し、200 ms条件が訂正作業と主観的負荷へ悪影響を与えることを示した。

論文の20 msをそのままIME内部予算にするのではなく、OSとアプリを含むend-to-end予算から逆算する。

| 計測点 | 初期エンジン予算 |
| --- | ---: |
| key down → preedit action生成 | p95 5 ms |
| Space → 最初の候補list生成 | p95 20 ms |
| 候補移動 → action生成 | p95 5 ms |
| cold dictionary open | p95 100 ms |

平均だけでなくp50、p95、p99、最大値を測る。長文、候補数、cold page、辞書破損、学習ファイル肥大化を分けてbenchmarkする。

## 13. 実験計画

### Experiment A: 最小統計baseline

- system dictionary: 10万、25万、50万entry
- model: unigram、class bigram、pruned word bigram
- decoder: exact Viterbi、beamあり
- output: Acc@1、Acc@5、CER、size、RSS、p95 latency

目的は最高精度ではなく、サイズと精度のPareto frontierを得ることである。

### Experiment B: 辞書表現

- flat sorted array
- LOUDS
- minimal FST
- mmapあり/なし
- 8/16-bit cost

同じ論理entryから生成し、検索APIと候補結果を完全一致させる。

### Experiment C: 学習安全性

- 同じ読みの候補選択
- 文節境界が変わる選択
- 誤操作を1回行った場合
- 10回繰り返した場合
- 30日相当の減衰後

改善数、副作用数、学習ファイルサイズ、lookup遅延を測る。

### Experiment D: 人間の訂正コスト

少人数の内部試験から始め、固定文と自由文を分ける。ローカル計測のみとし、同意なしに入力内容を収集しない。

- 完了時間
- 変換回数
- 候補移動数
- 文節変更数
- Backspace数
- 未訂正誤り
- 主観的な予測可能性

## 14. 文献から決められないこと

調査を増やしても、次は実装と計測なしには決められない。

- 10 MB以下で許容できる語彙数と変換品質
- Rust実装でLOUDSとFSTのどちらが速いか
- macOSの実アプリを含むend-to-end遅延
- 一般ユーザーが許容する候補順位と訂正回数
- 辞書生成物を配布できるライセンス構成
- 学習制約が実際の個人語彙を阻害しないか

また、商用IMEの最新モデル、学習データ、実測値の多くは非公開である。公開論文の比較だけでGoogle日本語入力、Apple日本語入力、Microsoft IME、ATOKとの製品品質差を断定できない。

## 15. 残っている研究上の穴

今回の追加調査後も、次の領域は公開研究が薄い、または今回未解決である。

1. デスクトップ日本語IMEでの訂正操作を含む大規模user study
2. 現代のmacOSアプリ横断でのmarked text・候補UI遅延
3. 小規模辞書に限定した最近のかな漢字変換benchmark
4. ユーザー学習を数か月使ったときの副作用と忘却
5. 方言、専門語、個人名を公平に扱う軽量model
6. 量子化したcostが候補順位へ与える影響
7. BCCWJ以外の再配布可能な学習・評価corpus

これらは「論文をさらに探せば必ず答えがある」とは限らない。Phase 1では、公開可能なbenchmarkと測定方法をこのプロジェクト自身の成果物として整備する価値がある。

## 16. 推奨する採用順序

### 初期採用

- 生成モデル
- class bigram
- 読みモデル
- Viterbi lattice
- 静的な圧縮辞書
- dynamic candidate generator
- must-pass + corpus + difficult case評価
- 学習を別layerへ隔離

### benchmark後に採用判断

- LOUDS対minimal FST
- phrase化
- entropy pruning
- 8/12-bit cost量子化
- typo候補
- word bigram reranker
- discriminative N-best reranker

### 初期不採用

- neural model標準同梱
- 自動typo確定
- 毎キー全文Transformer推論
- Webからの語彙自動収集
- 学習結果のsystem dictionaryへの直接merge
- 平均CERだけによる出荷判定

## 17. 主要参考文献

### かな漢字変換・製品設計

- 森信介ほか (1999): [確率的モデルによる仮名漢字変換](https://cir.nii.ac.jp/crid/1050845762815505920)
- 工藤拓ほか (2011): [統計的かな漢字変換システム Mozc](https://www.anlp.jp/proceedings/annual_meeting/2011/pdf_dir/C4-3.pdf)
- Tokunaga, Okanohara, Mori (2011): [Discriminative Method for Japanese Kana-Kanji Input Method](https://aclanthology.org/W11-3502/)
- Maeta, Mori (2012): [Statistical Input Method based on a Phrase Class n-gram Model](https://aclanthology.org/W12-4801/)

### 圧縮・データ構造

- Kudo et al. (2011): [Efficient dictionary and language model compression for input method editors](https://aclanthology.org/W11-3503/)
- Daciuk et al. (2000): [Incremental Construction of Minimal Acyclic Finite-State Automata](https://aclanthology.org/J00-1002/)
- Stolcke (1998): [Entropy-based Pruning of Backoff Language Models](https://arxiv.org/abs/cs/0006025)
- Heafield (2011): [KenLM: Faster and Smaller Language Model Queries](https://aclanthology.org/W11-2123/)

### 適応・未知語・誤入力

- Suzuki, Gao (2005): [A Comparative Study on Language Model Adaptation Techniques Using New Evaluation Metrics](https://aclanthology.org/H05-1034/)
- 森信介 (2006): [単語リストと生コーパスによる言語モデル適応](https://doi.org/10.5715/jnlp.13.4_33)
- 笹田鉄郎・森信介・河原達也 (2010): [自動獲得した未知語の読み・文脈情報による仮名漢字変換](https://doi.org/10.5715/jnlp.17.4_131)
- 山本喜大ほか (1997): [かな漢字変換における誤入力の訂正](https://cir.nii.ac.jp/crid/1573950401966931328)
- Kasahara et al. (2011): [Error Correcting Romaji-kana Conversion for Japanese Language Education](https://aclanthology.org/W11-3506/)
- 田中佑・村脇有吾・河原大輔・黒橋禎夫 (2021): [日本語Wikipediaの編集履歴に基づく入力誤りデータセットと訂正システムの構築](https://www.jstage.jst.go.jp/article/jnlp/28/4/28_995/_article/-char/ja/)

### ニューラル・低遅延

- Yao et al. (2019): [Enabling Real-time Neural IME with Incremental Vocabulary Selection](https://aclanthology.org/N19-2001/)
- Sarhangzadeh, Watanabe (2024): [Alignment-Based Decoding Policy for Low-Latency and Anticipation-Free Neural Japanese Input Method Editors](https://aclanthology.org/2024.findings-acl.479/)
- ensan (2025): [ニューラルかな漢字変換システム Zenzai](https://www.anlp.jp/proceedings/annual_meeting/2025/pdf_dir/P1-19.pdf)

### 評価・コーパス・HCI

- Soukoreff, MacKenzie (2003): [Metrics for text entry research](https://doi.org/10.1145/642611.642632)
- Schmid et al. (2023): [Effects of Text Input Latency on Performance and Task Load](https://doi.org/10.1145/3626705.3627784)
- Maekawa et al. (2014): [Balanced corpus of contemporary written Japanese](https://link.springer.com/article/10.1007/s10579-013-9261-0)
- azooKey: [AJIMEE-Bench](https://github.com/azooKey/AJIMEE-Bench)

## 18. 読み方に関する注意

- 論文の精度値は、corpus、語彙、正解表記、入力長、候補生成器が異なるため横並びの製品比較ではない。
- 古い研究でも、学習副作用や訂正interfaceの問題は現在のIMEに残る。
- neural研究の推論時間は、モデル配布サイズ、起動時間、メモリ、OS integrationを含まない場合がある。
- benchmarkの高得点は、候補UI、アプリ互換性、install体験を保証しない。
- ライセンスの記述は技術調査であり、製品配布前には個別確認が必要である。
