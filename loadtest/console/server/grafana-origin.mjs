export function grafanaProxyPort(){return Number(process.env.GRAFANA_PROXY_PORT||Number(process.env.PORT||4174)+2);}
export function grafanaEmbedOrigin(){
 const consoleUrl=new URL(process.env.ORIGIN||'http://127.0.0.1:4174');
 const fallback=new URL(consoleUrl);fallback.port=String(grafanaProxyPort());
 const embed=new URL(process.env.GRAFANA_EMBED_ORIGIN||fallback.origin);
 if(!['http:','https:'].includes(embed.protocol)||embed.username||embed.password||embed.origin===consoleUrl.origin||embed.pathname!=='/'||embed.search||embed.hash)throw new Error('GRAFANA_EMBED_ORIGIN must be a separate HTTP(S) origin');
 return embed.origin;
}
