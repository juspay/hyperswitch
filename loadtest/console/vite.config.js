import { defineConfig, loadEnv } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

export default defineConfig(({mode}) => {
  for(const [key,value] of Object.entries(loadEnv(mode,process.cwd(),''))) process.env[key] ??= value;
  return {plugins:[sveltekit()],server:{host:'127.0.0.1'},build:{target:'es2022'}};
});
