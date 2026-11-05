import semanticRelease from 'semantic-release';
import { appendFileSync } from 'node:fs';
const result = await semanticRelease();
if (result) {
  const isPrerelease = Boolean(result.nextRelease.channel);
  appendFileSync(
    process.env.GITHUB_OUTPUT,
    `tag=${result.nextRelease.gitTag}\nversion=${result.nextRelease.version}\nis_prerelease=${isPrerelease}\n`,
  );
}
