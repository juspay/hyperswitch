export const settingsSections=[['environment','Environment'],['connections','Connections'],['database','Database & TLS'],['load','Load generation'],['results','Results limits'],['workers','Workers'],['grafana','Grafana']];
export const settingsSection=value=>settingsSections.some(([id])=>id===value)?value:'connections';
export const settingsHref=(section='connections')=>`/settings?section=${settingsSection(section)}`;
