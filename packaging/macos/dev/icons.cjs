// Regenerate the developer Mac icon. Tooling only: Node + sharp; macOS iconutil for .icns.
// Uses the existing MIT/Apache-2.0 owl vector, a teal field and original geometric DEV lettering.
const fs = require('node:fs/promises');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const sharp = require('sharp');

async function main() {
  const root = path.resolve(__dirname, '../../..');
  const assets = path.join(root, 'assets/app-icon');
  const canonical = await fs.readFile(path.join(assets, 'filmcraft.svg'), 'utf8');
  if (!canonical.includes('fill="#8b5cf6"') || !canonical.includes('viewBox="0 0 512 512"')) {
    throw new Error('Canonical icon geometry/palette changed; review the developer variant.');
  }
  // Letters are paths drawn here, so rendering requires no proprietary or system font.
  const badge = `<g><rect x="264" y="358" width="218" height="104" rx="24" fill="#061f2b" stroke="#f5fff9" stroke-width="6"/>
    <path d="M294 382V438H309C339 438 339 382 309 382Z M379 382H349V438H379 M349 410H374 M399 382L419 438L439 382"
      fill="none" stroke="#f5fff9" stroke-width="10" stroke-linecap="round" stroke-linejoin="round"/></g>`;
  const svg = canonical.replace('fill="#8b5cf6"', 'fill="#08b3a7"')
    .replace('viewBox="0 0 512 512"', 'viewBox="-62.14 -62.14 636.27 636.27"')
    .replace('</svg>', badge + '</svg>');
  const iconset = path.join(root, 'target/filmcraft-dev.iconset');
  await fs.mkdir(iconset, { recursive: true });
  const render = (size, file) => sharp(Buffer.from(svg), { density: 144 }).resize(size, size).png().toFile(file);
  for (const size of [16, 32, 128, 256, 512]) {
    await render(size, path.join(iconset, `icon_${size}x${size}.png`));
    await render(size * 2, path.join(iconset, `icon_${size}x${size}@2x.png`));
  }
  await fs.copyFile(path.join(iconset, 'icon_512x512.png'), path.join(assets, 'filmcraft-dev-macos-512.png'));
  execFileSync('/usr/bin/iconutil', ['-c', 'icns', '-o', path.join(assets, 'filmcraft-dev.icns'), iconset], { stdio: 'inherit' });
  console.log('Developer Dock icon and bundle icon regenerated.');
}
main().catch(error => { console.error(error.message); process.exitCode = 1; });
