import { passwordHash } from '../server/auth.mjs';
let password = '';
for await (const chunk of process.stdin) password += chunk;
password = password.trimEnd();
if (password.length < 12) throw new Error('Use at least 12 characters');
console.log(await passwordHash(password));
