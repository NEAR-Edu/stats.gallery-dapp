const ONE_DAY = 1_000_000_000 * 60 * 60 * 24;

// 1e24, calculated like this because JS numbers don't work that large
const ONE_NEAR = BigInt(1e12) ** 2n;

const owner_id = process.env['OWNER_ID'];

console.log(
  JSON.stringify({
    owner_id,
    proposal_duration: ONE_DAY * 7 + '', // 7 days
    badge_rate_per_day: ONE_NEAR / 10n + '', // 0.1 NEAR
    badge_max_active_duration: ONE_DAY * 90 + '', // 90 days
    badge_min_creation_deposit: (ONE_NEAR * 5n) / 2n + '', // 2.5 NEAR
  }),
);
