#!/usr/bin/env python3
"""Generate UI identities/auth labels from routing enum and sandbox definitions."""
import json,re,pathlib,tomllib,sys
root=pathlib.Path(__file__).resolve().parents[3]
source=(root/'crates/euclid/src/enums.rs').read_text().split('pub enum RoutableConnectors {',1)[1].split('\n}',1)[0]
config=tomllib.loads((root/'crates/connector_configs/toml/sandbox.toml').read_text())
entries=[];attrs=[]
for line in source.splitlines():
 line=line.strip()
 if line.startswith('#['):attrs.append(line);continue
 m=re.fullmatch(r'(\w+),',line)
 if not m:continue
 variant=m[1];serialized=re.search(r'serde\(rename = "([^"]+)"', '\n'.join(attrs))
 name=serialized[1] if serialized else re.sub(r'(?<!^)(?=[A-Z][a-z])','_',variant).lower()
 dummy=any('dummy_connector' in a for a in attrs);attrs=[]
 c=config.get(name,{})
 auth=c.get('connector_auth',{});auth_type=next(iter(auth),None)
 fields=[]
 if auth_type and isinstance(auth[auth_type],dict):
  fields=[{'name':k,'label':v if isinstance(v,str) else k.replace('_',' ').title()} for k,v in auth[auth_type].items()]
 entries.append({'id':name,'label':name.replace('_',' ').title(),'dummy':dummy,'card':dummy or bool(c.get('credit')),'networks':[row['payment_method_type'] for row in c.get('credit',[]) if isinstance(row,dict) and isinstance(row.get('payment_method_type'),str)],'authType':auth_type or ('HeaderKey' if dummy else None),'fields':fields or ([{'name':'api_key','label':'Test key'}] if dummy else [])})
output=json.dumps(entries,indent=2)+'\n';target=root/'loadtest/console/src/connector-catalog.json'
if '--check' in sys.argv:
 if not target.exists() or target.read_text()!=output:sys.exit('Connector catalog is stale; run python3 scripts/connector-catalog.py')
else:target.write_text(output)
print(f'{len(entries)} routing connectors')
