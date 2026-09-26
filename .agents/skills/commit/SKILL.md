---
name: commit-all
description: main, developともコミットする
disable-model-invocation: false
---

commitを以下の手順で実行してください。
1.前回のコミットとの差分を調べる
2.差分のあるファイルを抽出する
3.コミットする
4.developにいるときは、git co main, mainにいるときは、git co developしてgit merge <commited-branch>
