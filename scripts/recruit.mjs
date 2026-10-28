// Refresh data/recruits.json with the most-liked #donkeysofinstagram posts using Apify's
// Instagram Hashtag Scraper actor. Needs your own token: APIFY_TOKEN=... node scripts/recruit.mjs
//
// Note: the committed recruits.json is sample data with AI-generated donkeys, so the repo
// doesn't redistribute anyone's real photos. If you run this, keep the results local.
import fs from 'node:fs';

const token = process.env.APIFY_TOKEN;
if (!token) {
  console.error('Set APIFY_TOKEN first (https://console.apify.com/account/integrations).');
  process.exit(1);
}

const run = await fetch(`https://api.apify.com/v2/acts/apify~instagram-hashtag-scraper/run-sync-get-dataset-items?token=${token}`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ hashtags: ['donkeysofinstagram'], resultsLimit: 30 }),
});
if (!run.ok) throw new Error(`Apify returned ${run.status}`);
const posts = await run.json();

const recruits = posts
  .sort((a, b) => (b.likesCount ?? 0) - (a.likesCount ?? 0))
  .slice(0, 10)
  .map((p) => ({
    handle: `@${p.ownerUsername}`,
    photo: p.displayUrl,
    caption: (p.caption ?? '').slice(0, 140),
    likes: p.likesCount ?? 0,
    comments: p.commentsCount ?? 0,
  }));

fs.writeFileSync(new URL('../data/recruits.local.json', import.meta.url), JSON.stringify(recruits, null, 2));
console.log(`Saved ${recruits.length} recruits to data/recruits.local.json. Let's get some asses in here.`);
