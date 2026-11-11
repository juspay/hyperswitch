import { defineConfig, loadEnv } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';
import {startGrafanaProxy} from './server/grafana-embed.mjs';

export default defineConfig(({mode}) => {
  for(const [key,value] of Object.entries(loadEnv(mode,process.cwd(),''))) process.env[key] ??= value;
  return {plugins:[sveltekit(),{name:'isolated-grafana',async configureServer(server){const proxy=await startGrafanaProxy();server.httpServer?.once('close',()=>proxy.close());}}],server:{host:'127.0.0.1',watch:{ignored:['**/.data/**']}},build:{target:'es2022'}};
});
