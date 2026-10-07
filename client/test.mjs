import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createRequire} from 'node:module';
import {createOnchainClient,decodePlanet,layout,planetRent} from './onchain.mjs';
const require=createRequire(import.meta.url), web3=require(process.env.SOLANA_WEB3_PATH || '@solana/web3.js');
const api=createOnchainClient(web3), fixture=JSON.parse(readFileSync(new URL('../onchain-results.json',import.meta.url)));
const owner=new web3.PublicKey(fixture.owner),planet=api.planet(owner,5n);
assert.equal(planet.toBase58(),fixture.planet);assert.equal(api.engine(planet).toBase58(),fixture.engine);assert.equal(api.topology.toBase58(),fixture.topology);
assert.equal(layout.planetBytes,fixture.planetBytes);assert.equal(layout.engineBytes,fixture.engineBytes);
const decoded=decodePlanet(Buffer.from(fixture.planetData,'hex'));assert.equal(decoded.population,10n);assert.equal(decoded.phase,3n);assert.equal(decoded.houses,2n);assert.equal(decoded.buildings.length,4);assert.equal(decoded.workers,5n);
assert.equal(planetRent,525056n);
const tx=new web3.Transaction({feePayer:owner,recentBlockhash:web3.SystemProgram.programId.toBase58()});
const setup=api.initializeAndDelegate(owner,planetRent,owner);assert.equal(setup.length,2);tx.add(...setup);assert.ok(tx.serializeMessage().length>0);
for(const ix of [api.createPlanet(owner,owner,0n,23,web3.Keypair.generate().publicKey),api.prepareTopology(owner),api.build(owner,planet,12,2,0),api.demolish(owner,planet,12),api.setPaused(owner,planet,12,true),api.advance(planet),api.closePlanet(owner,owner,planet),api.closeTopology(owner),api.requestSponsorUndelegation(owner),api.reclaimSponsor(owner)]){const t=new web3.Transaction({feePayer:owner,recentBlockhash:web3.SystemProgram.programId.toBase58()}).add(ix);assert.ok(t.serializeMessage().length>0);}
console.log('Client: instruction serialization, atomic sponsor setup, Rust PDA/layout parity and account decoding pass.');
const session=web3.Keypair.generate().publicKey,admin=web3.Keypair.generate().publicKey;
for(const ix of [api.createWithSession(owner,admin,session,23),api.linkSession(owner,admin,session)]){
 assert.ok(!ix.keys.some(k=>k.pubkey.equals(owner)&&k.isSigner));
 assert.ok(ix.keys.some(k=>k.pubkey.equals(session)&&k.isSigner));
 assert.ok(ix.keys.some(k=>k.pubkey.equals(admin)&&k.isSigner));
}
