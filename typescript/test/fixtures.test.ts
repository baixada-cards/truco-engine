import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

import { executeFixture, type EngineFixture } from '../src/index.ts'

const FIXTURES_DIR = fileURLToPath(
  new URL('../../.cache/truco-spec/engine/fixtures/', import.meta.url),
)
const FIXTURE_PATHS = collectFixturePaths(FIXTURES_DIR)

test('TypeScript engine fixture corpus size matches the Rust runner corpus', () => {
  assert.equal(FIXTURE_PATHS.length, 67)
})

for (const fixturePath of FIXTURE_PATHS) {
  test(`TypeScript engine passes ${relative(FIXTURES_DIR, fixturePath)}`, () => {
    const fixture = JSON.parse(readFileSync(fixturePath, 'utf8')) as EngineFixture
    const report = executeFixture(fixture)

    assert.equal(report.fixture_id, fixture.id)
    assert.equal(report.status, 'pass', report.message)
  })
}

function collectFixturePaths(dir: string): string[] {
  const paths: string[] = []
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const path = `${dir}/${entry.name}`
    if (entry.isDirectory()) {
      paths.push(...collectFixturePaths(path))
    } else if (entry.isFile() && entry.name.endsWith('.json')) {
      paths.push(path)
    }
  }
  return paths.sort()
}
