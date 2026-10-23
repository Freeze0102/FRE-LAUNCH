// Live, read-only smoke check of the exact Minecraft release used in bug reports.
// This exercises Mojang metadata, index integrity, and one actual asset download.
// It does not claim to run Minecraft or invoke the FRE LAUNCH GUI.
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const RELEASE = process.env.FRE_SMOKE_VERSION || "1.21.10";
const manifestUrl = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
async function getBytes(url) {
  const parsed = new URL(url);
  assert.equal(parsed.protocol, "https:", "Minecraft downloads must use HTTPS");
  assert.ok([
    "piston-meta.mojang.com", "piston-data.mojang.com",
    "resources.download.minecraft.net", "launchermeta.mojang.com",
    "launcher.mojang.com"
  ].includes(parsed.hostname), `Unexpected Mojang host: ${parsed.hostname}`);
  let last = "";
  for (let attempt = 0; attempt < 3; attempt++) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(30000) });
      if (!response.ok) throw Error(`HTTP ${response.status} for ${url}`);
      return Buffer.from(await response.arrayBuffer());
    } catch (error) {
      last = String(error);
      if (attempt < 2) await new Promise(resolve => setTimeout(resolve, 500 * (attempt + 1)));
    }
  }
  throw Error(`Mojang smoke download failed: ${last}`);
}
function sha1(buffer) { return createHash("sha1").update(buffer).digest("hex"); }
function checkHash(bytes, digest, name) {
  assert.match(digest, /^[a-f0-9]{40}$/i, `${name} has no valid SHA1`);
  assert.equal(sha1(bytes).toLowerCase(), digest.toLowerCase(), `${name} SHA1 mismatch`);
}
const manifest = JSON.parse((await getBytes(manifestUrl)).toString("utf8"));
const chosen = manifest.versions?.find(v => v.id === RELEASE);
assert.ok(chosen, `Minecraft ${RELEASE} missing from official manifest`);
const profileBytes = await getBytes(chosen.url);
if (chosen.sha1) checkHash(profileBytes, chosen.sha1, "release manifest entry");
const profile = JSON.parse(profileBytes.toString("utf8"));
assert.equal(profile.id, RELEASE);
assert.ok(profile.downloads?.client?.sha1, "Minecraft client metadata missing");
assert.ok(profile.downloads?.client?.url, "Minecraft client URL missing");
assert.ok(profile.javaVersion?.majorVersion >= 8, "Java requirement missing");
assert.ok(Array.isArray(profile.libraries) && profile.libraries.length > 0, "Minecraft libraries missing");
const indexMeta = profile.assetIndex;
assert.ok(indexMeta?.url && indexMeta?.sha1, "Minecraft asset index metadata missing");
const indexBytes = await getBytes(indexMeta.url);
assert.ok(indexBytes.length < 40 * 1024 * 1024, "Minecraft asset index is too large");
checkHash(indexBytes, indexMeta.sha1, "asset index");
const index = JSON.parse(indexBytes.toString("utf8"));
const sample = Object.values(index.objects ?? {}).find(asset =>
  typeof asset.hash === "string" && asset.size > 0 && asset.size < 16_000
);
assert.ok(sample, "No small asset available for a network smoke test");
const assetUrl = `https://resources.download.minecraft.net/${sample.hash.slice(0,2)}/${sample.hash}`;
const assetBytes = await getBytes(assetUrl);
checkHash(assetBytes, sample.hash, "real Minecraft asset");
console.log(`Mojang smoke OK: ${RELEASE} / Java ${profile.javaVersion.majorVersion} / ${profile.libraries.length} libraries / ${Object.keys(index.objects).length} assets / sample asset SHA-1 verified`);
