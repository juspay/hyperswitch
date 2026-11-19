const test = require('node:test');
const assert = require('node:assert/strict');
const {inheritedEnvironmentArgs,loadYaml} = require('../../lib/config');
const path = require('node:path');
test('secret environment inheritance keeps values out of container arguments',()=>{
 assert.deepEqual(inheritedEnvironmentArgs(['KRONOS_ENCRYPTION_KEY'],{KRONOS_ENCRYPTION_KEY:'private-test-value'}),['-e','KRONOS_ENCRYPTION_KEY']);
 assert.throws(()=>inheritedEnvironmentArgs(['KRONOS_ENCRYPTION_KEY'],{}),/not set/);
 assert.throws(()=>inheritedEnvironmentArgs(['INVALID=VALUE'],{}),/Invalid/);
 const config=loadYaml(path.resolve(__dirname,'../../deploy/config.docker-hub.yaml'));
 assert.ok(Object.values(config.application_services).some(service=>service.env_from?.includes('KRONOS_ENCRYPTION_KEY')));
 assert.ok(Object.values(config.application_services).every(service=>!service.env?.KRONOS_ENCRYPTION_KEY));
});
