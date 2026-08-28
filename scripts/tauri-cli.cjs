// Wrapper to run the Tauri CLI without its argv[0] sniffing.
// The harness node binary is named "DSH Desktop", which tauri.js
// mis-parses as a CLI argument. We call cli.run() directly instead.
// Usage: node scripts/tauri-cli.cjs <command...>
const cli = require('@tauri-apps/cli/main')

const args = process.argv.slice(2)
if (args.length === 0) {
  console.error('usage: node scripts/tauri-cli.cjs <command...>')
  process.exit(1)
}

cli.run(args, `tauri ${args.join(' ')}`).catch((err) => {
  cli.logError(err.message)
  process.exit(1)
})
