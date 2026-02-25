import io
from maenifold import main
from maenifold.config import Config


def test_payload_file_support(tmp_path, capsys):
    Config.override_root(str(tmp_path))
    payload_path = tmp_path / "payload.json"
    payload_path.write_text('{"title":"file-test","content":"[[file-concept]]"}', encoding="utf-8")

    exit_code = main.main(["--tool", "WriteMemory", "--payload", f"@{payload_path}"])
    out = capsys.readouterr().out

    Config.reset_overrides()

    assert exit_code == 0
    assert "Created memory FILE" in out


def test_payload_stdin_support(tmp_path, capsys, monkeypatch):
    Config.override_root(str(tmp_path))
    monkeypatch.setattr("sys.stdin", io.StringIO('{"title":"stdin-test","content":"[[stdin-concept]]"}'))

    exit_code = main.main(["--tool", "WriteMemory", "--payload", "@-"])
    out = capsys.readouterr().out

    Config.reset_overrides()

    assert exit_code == 0
    assert "Created memory FILE" in out
