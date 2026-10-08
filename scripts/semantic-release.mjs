import semanticRelease from 'semantic-release';
import { appendFileSync } from 'node:fs';
const result = await semanticRelease();
if (result) appendFileSync(process.env.GITHUB_OUTPUT, `tag=${result.nextRelease.gitTag}\nversion=${result.nextRelease.version}\n`);
