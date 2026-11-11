import { scrypt, randomBytes, createHash, timingSafeEqual } from 'node:crypto';
import { promisify } from 'node:util';
import { controlDb } from './state.mjs';
import { HttpError } from './errors.mjs';
const derive = promisify(scrypt);
export const sessionCookie = 'loadtest_session';
const digest = token => createHash('sha256').update(token).digest('hex');
let passwordChecks=0;

export async function passwordHash(password) {
  const salt = randomBytes(16).toString('hex');
  return `scrypt:${salt}:${(await derive(password,salt,64)).toString('hex')}`;
}
export async function verifyPassword(password, encoded) {
  const [algorithm,salt,key] = (encoded || '').split(':');
  if (algorithm !== 'scrypt' || !/^[a-f0-9]{32}$/.test(salt || '') || !/^[a-f0-9]{128}$/.test(key || '')) return false;
  const actual = await derive(password,salt,64);
  return timingSafeEqual(actual,Buffer.from(key,'hex'));
}
export function authenticated(token) {
  if (!token) return false;
  return Boolean(controlDb().prepare('SELECT token FROM sessions WHERE token=? AND expires>?').get(digest(token),Date.now()));
}
export async function login(username,password,ip) {
  const db = controlDb(), now = Date.now(), key = digest(ip);
  if(passwordChecks>=8)throw new HttpError(429,'Login is busy. Try again shortly.');
  // Reserve before yielding to scrypt, atomically across web processes too.
  const reserved=db.prepare(`INSERT INTO login_limits VALUES(?,1,?) ON CONFLICT(key) DO UPDATE SET
    failures=CASE WHEN reset_at<=? THEN 1 ELSE failures+1 END,
    reset_at=CASE WHEN reset_at<=? THEN excluded.reset_at ELSE reset_at END
    WHERE reset_at<=? OR failures<5 RETURNING key`).get(key,now+900000,now,now,now);
  if(!reserved)throw new HttpError(429,'Too many attempts. Try again in 15 minutes.');
  let valid=false;passwordChecks++;
  try{if(password.length<=4096&&username.length<=256)valid=await verifyPassword(password,process.env.ADMIN_PASSWORD_HASH);}
  finally{passwordChecks--;}
  if (username !== (process.env.ADMIN_USERNAME || 'admin') || !valid) {
    return null;
  }
  const token = randomBytes(32).toString('hex');
  db.prepare('DELETE FROM sessions WHERE expires<=?').run(now);
  db.prepare('DELETE FROM login_limits WHERE key=?').run(key);
  db.prepare('INSERT INTO sessions VALUES(?,?)').run(digest(token),now+8*3600000);
  return token;
}
export function logout(token) { if(token) controlDb().prepare('DELETE FROM sessions WHERE token=?').run(digest(token)); }
export function sameOrigin(request,url) {
  const origin = request.headers.get('origin');
  return origin === (process.env.ORIGIN || url.origin);
}
