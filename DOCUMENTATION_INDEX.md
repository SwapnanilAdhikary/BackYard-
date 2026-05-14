# Documentation Index

## 📚 For Users (Start Here!)

| File | Read Time | Purpose |
|------|-----------|---------|
| [QUICK_START.md](QUICK_START.md) | 2 min | 30-second copy-paste example |
| [USAGE.md](USAGE.md) | 15 min | Complete user guide with 10+ patterns |
| [TESTING.md](TESTING.md) | 10 min | How to test your jobs |
| [CRATE_GUIDE.md](CRATE_GUIDE.md) | 10 min | API reference & feature guide |
| [CRATE_READY.txt](CRATE_READY.txt) | 5 min | Readiness summary |

**→ Start with [QUICK_START.md](QUICK_START.md)**

## 📖 For Project Understanding

| File | Purpose |
|------|---------|
| [README.md](README.md) | Architecture, features, design patterns |
| [CLAUDE.md](CLAUDE.md) | Implementation details, development notes |

## 🧪 Examples (Working Code)

| File | Shows |
|------|-------|
| [examples/user_test.rs](backyard-sqlite/tests/user_test.rs) | ✅ All 4 test patterns working |
| [examples/basic_sqlite.rs](examples/basic_sqlite.rs) | Simple end-to-end workflow |
| [examples/quick_test.rs](examples/quick_test.rs) | Minimal working example |
| [examples/testing_guide.rs](examples/testing_guide.rs) | Testing patterns & harness |

## 🔍 In-Code Documentation

Run: `cargo doc --open`

- `backyard` - Main facade with full documentation
- `backyard_core::Job` - Job trait with examples
- `backyard_core::Queue` - Queue trait documentation
- All public types have rustdoc comments

## 📋 Quick Navigation

### "I'm new, where do I start?"
→ [QUICK_START.md](QUICK_START.md) (2 min)

### "I want to understand the full API"
→ [USAGE.md](USAGE.md) (15 min)

### "How do I test my jobs?"
→ [TESTING.md](TESTING.md) (10 min)

### "What's the architecture?"
→ [README.md](README.md) + [CLAUDE.md](CLAUDE.md)

### "Can I use it now?"
→ [CRATE_READY.txt](CRATE_READY.txt) (Yes! ✅)

### "Show me working code"
→ Run: `cargo test --test user_test -- --nocapture`

## 🎯 Common Tasks

### Define a job
→ See [USAGE.md](USAGE.md) section "Define Your First Job"

### Enqueue a job
→ See [USAGE.md](USAGE.md) section "Enqueue a Job"

### Run workers
→ See [USAGE.md](USAGE.md) section "Run the Worker"

### Handle errors
→ See [USAGE.md](USAGE.md) section "Error Handling & Retries"

### Test jobs
→ See [TESTING.md](TESTING.md) section "Automated Test Examples"

### Full working example
→ See [examples/user_test.rs](backyard-sqlite/tests/user_test.rs)

## ✅ What's Production-Ready

- ✅ Job definition & execution
- ✅ SQLite backend (no external deps)
- ✅ Automatic retries with backoff
- ✅ Queue inspection
- ✅ Multiple queues & priorities
- ✅ Scheduled jobs
- ✅ Testing infrastructure
- ✅ Complete documentation
- ✅ Integration tests passing

## ⏳ Planned for v0.2+

- Redis backend (design complete, implementation pending)
- Web UI endpoints
- Cron integration

## 🚀 To Get Started

```bash
# 1. Read the quick start
cat QUICK_START.md

# 2. See working tests
cargo test --test user_test -- --nocapture

# 3. Browse the API
cargo doc --open

# 4. Build your app!
```

---

**Ready to use? Pick a file from the top table and start reading!** 📖
