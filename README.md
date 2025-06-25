# TARDIS

![Crates.io](https://img.shields.io/crates/v/tardis)
![Docs.rs](https://img.shields.io/docsrs/tardis)

![Tardis logo](./assets/tardis.png)

> **TARDIS** – *Time And Relative Date Input Simplifier*.
> Like the Doctor’s ship in *Doctor Who*, it translates human‑friendly time
> expressions into precise datetimes right from your terminal.

---

## ✨ Features

- Parse expressions such as
  `next Monday at 09:00`, `in 2 hours`, `tomorrow 14:30`
- Output using custom formats (`--format`)
- Convert or force output time‑zones (`--timezone`)
- Commented `TOML` configuration with named presets
- Designed for pipes, automation and scripting
- Cross‑platform (Linux · macOS · Windows)

---

## 📦 Installation

```bash
cargo install tardis --locked
```

> **Why `--locked`?**
> Guarantees the same dependency versions used in continuous
> integration, avoiding surprises from newer transitive crates.

---

## 🚀 Quick start

```bash
td "tomorrow 15:00"
td "next Friday" --format br
td "in 2 hours" --timezone Europe/London
echo "next Monday at 09:00" | td
```

---

## ⚙️ Configuration

On first run, a config file is generated automatically:

| Platform                          | Path                              |
|----------------------------------|-----------------------------------|
| `$XDG_CONFIG_HOME` is set        | `$XDG_CONFIG_HOME/tardis/config.toml` |
| Linux (default)                  | `~/.config/tardis/config.toml`    |
| macOS                            | `~/Library/Application Support/tardis/config.toml` |
| Windows                          | `%APPDATA%\tardis\config.toml`  |

Precedence: **CLI flags → Environment variables → Config file**.

```toml
# config.toml
format = "%Y-%m-%dT%H:%M:%S%:z"
timezone = ""            # empty = use local OS time‑zone

[formats]
taskline = "%d.%m.%Y %H:%M"
br       = "%d/%m/%Y"
iso      = "%Y-%m-%dT%H:%M:%S"
```

---

## 🛠 Options

```text
-f, --format     chrono format string or preset name
-t, --timezone   output time‑zone (IANA name)
--now            reference datetime (RFC3339) for deterministic output
--help           show CLI usage
--version        show binary version
```

---

## 🔁 Pipes & automation

```bash
# Schedule a reminder in TaskLine (example)
tl t "Prepare slides" -d $(td "next Friday at 12:00" -f taskline)
```

---

## 🌐 Environment variables

| Variable           | Purpose                                              |
|--------------------|------------------------------------------------------|
| `TARDIS_FORMAT`    | fallback format when `--format` is omitted           |
| `TARDIS_TIMEZONE`  | fallback time‑zone when `--timezone` is omitted      |
| `XDG_CONFIG_HOME`  | overrides default config directory                   |

---

## 📄 License

MIT


---

## 🤝 Contributing

Bug reports and pull requests are welcome!
See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for guidelines.

---

## 🧠 Trivia

The name **TARDIS** pays homage to the iconic, bigger‑on‑the‑inside
time machine from *Doctor Who*. This CLI helps you navigate time too —
minus the wibbly‑wobbly stuff.
