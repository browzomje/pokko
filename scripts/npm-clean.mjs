import { spawn } from 'node:child_process';

// npm exports this obsolete option into lifecycle scripts, then warns on nested npm.
const environment = { ...process.env };
for (const key of Object.keys(environment)) {
  if (key.toLowerCase() === 'npm_config_global_ignore_file') delete environment[key];
}
const task = process.argv[2];
if (!['dev', 'build'].includes(task)) throw new Error('Expected dev or build');
const npmScript = process.env.npm_execpath;
const child = npmScript
  ? spawn(process.execPath, [npmScript, 'run', task], { env: environment, stdio: 'inherit' })
  : spawn(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['run', task], {
      env: environment, stdio: 'inherit', shell: process.platform === 'win32',
    });
for (const signal of ['SIGINT', 'SIGTERM']) process.on(signal, () => child.kill(signal));
child.on('error', (error) => { console.error(error.message); process.exitCode = 1; });
child.on('exit', (code, signal) => { process.exitCode = code ?? (signal ? 1 : 0); });
