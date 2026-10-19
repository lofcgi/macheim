import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const json = path => JSON.parse(readFileSync(path, 'utf8'));
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const pkg = json('package.json');
const lock = json('package-lock.json');
const tauri = json('src-tauri/tauri.conf.json');
const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8').match(/^version = "([^"]+)"/m)?.[1];
const cargoLock = readFileSync('src-tauri/Cargo.lock', 'utf8').match(/name = "macheim"\nversion = "([^"]+)"/)?.[1];
for (const version of [lock.version, lock.packages[''].version, tauri.version, cargo, cargoLock]) assert.equal(version, pkg.version, 'All app and lockfile versions must match');
assert.ok(readFileSync('src/components/setup/SetupWizard.tsx', 'utf8').includes(`Macheim v${pkg.version}`), 'Setup version label must match');
if (process.env.GITHUB_REF_TYPE === 'tag') assert.equal(process.env.GITHUB_REF_NAME, `v${pkg.version}`, 'Tag must match packaged version');
assert.equal(tauri.bundle.macOS.signingIdentity, '-', 'Ad-hoc signing required');
assert.deepEqual(tauri.plugins.updater.endpoints, ['https://github.com/lofcgi/macheim/releases/latest/download/latest.json'], 'Updater must read the latest GitHub release');
assert.match(Buffer.from(tauri.plugins.updater.pubkey, 'base64').toString(), /^untrusted comment: minisign public key/, 'Updater public key must be a minisign key');
assert.equal(json('src-tauri/tauri.release.conf.json').bundle.createUpdaterArtifacts, true, 'Release builds must produce signed updater artifacts');
assert.ok(readFileSync('.github/workflows/release.yml', 'utf8').includes('--config src-tauri/tauri.release.conf.json'), 'Release workflow must use the updater release config');

const catalog = json('compatibility/catalog.json');
const pluginSource = readFileSync('tools/item-material-compat/ItemMaterialCompat.cs', 'utf8');
assert.ok(pluginSource.includes(`"${catalog.plugin_version}")]`), 'Plugin/catalog versions match');
assert.ok(pluginSource.includes(`gameVersion == "${catalog.game_version}"`), 'Game version guard matches catalog');
assert.ok(pluginSource.includes(`Application.unityVersion == "${catalog.unity_version}"`), 'Unity version guard matches catalog');
const dll = 'src-tauri/resources/compatibility/Macheim.ItemMaterialCompat.dll';
const sources = ['ItemMaterialCompat.cs', 'ShaderPolicy.cs', 'AdditiveAlpha.cs', 'ItemMaterialCompat.csproj'].map(name => `tools/item-material-compat/${name}`);
const manifest = { plugin_version: catalog.plugin_version, dll, sha256: hash(dll), sources: Object.fromEntries(sources.map(path => [path, hash(path)])) };
const manifestPath = 'compatibility/plugin-manifest.json';
if (process.argv.includes('--record-plugin-build')) {
  // Generated build provenance; no proprietary reference assemblies are distributed.
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
} else assert.deepEqual(json(manifestPath), manifest, 'Rebuild and record the plugin after changing its source');
for (const rule of catalog.rules) {
  assert.ok(rule.prefabs.length > 0);
  assert.match(rule.version, /^\d+\.\d+\.\d+$/);
  for (const prefab of rule.prefabs) assert.match(prefab, /^[A-Za-z0-9_]+$/);
}
console.log(`Release ${pkg.version}: versions, catalog, plugin source provenance and SHA-256 verified.`);
