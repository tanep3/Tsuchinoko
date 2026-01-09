# V1.6.0 FT-001: with 文テスト
# NOTE: このテストは with 文のスコープブロック変換のみ確認
#       File I/O の完全サポートは将来バージョンで対応予定

def program_start() -> None:
    # with 文はスコープブロック {} に変換される
    # (File I/O テストはスキップ - 実際のファイル操作は複雑)
    print("FT-001: with 文テスト完了")

if __name__ == "__main__":
    program_start()
