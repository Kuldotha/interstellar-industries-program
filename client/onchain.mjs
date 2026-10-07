const text = s => new TextEncoder().encode(s);
const integer = (value, bytes) => { value = BigInt(value); if (value < 0n || value >= 1n << BigInt(bytes * 8)) throw Error('Integer out of range'); const out = new Uint8Array(bytes); for (let i = 0; i < bytes; i++, value >>= 8n) out[i] = Number(value & 255n); return out; };
const concat = (...parts) => { const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0)); let offset = 0; for (const p of parts) { out.set(p, offset); offset += p.length; } return out; };
export const layout = Object.freeze({planetBytes:6360,legacyPlanetBytes:6320,needsOffset:6320,engineBytes:9968,sponsorBytes:56,topologyBytes:23152,clockOffset:344,tierOffset:1032,tilesOffset:1152,tileBytes:8});
export const ephemeralRent = bytes => (BigInt(bytes) + 60n) * 32n;
export const planetRent = ephemeralRent(layout.planetBytes) + ephemeralRent(layout.engineBytes);
export function createOnchainClient({PublicKey, TransactionInstruction, SystemProgram}, programId = '4yysd21qfEAwjt19XiMdYL1GrRytMaBQxUZdiVx1Z5zd') {
 const program = new PublicKey(programId), magic = new PublicKey('Magic11111111111111111111111111111111111111'), context = new PublicKey('MagicContext1111111111111111111111111111111'), vault = new PublicKey('MagicVau1t999999999999999999999999999999999'), delegation = new PublicKey('DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh');
 const pda = (seeds, owner = program) => PublicKey.findProgramAddressSync(seeds, owner)[0];
 const meta = (pubkey, isWritable = false, isSigner = false) => ({pubkey, isWritable, isSigner});
 const ix = (tag, keys, ...data) => new TransactionInstruction({programId:program, keys, data:concat(integer(tag,8), ...data)});
 const sponsor = admin => pda([text('sponsor'), admin.toBytes()]);
 const planet = (owner,nonce) => pda([text('planet'),owner.toBytes(),integer(nonce,8)]);
 const engine = planet => pda([text('engine'),planet.toBytes()]);
 const topology = pda([text('topology'),integer(8,4)]);
 const runtime = planet => [meta(planet,true),meta(engine(planet),true),meta(context,true),meta(magic)];
 const action = (owner,planet) => [meta(owner,false,true),meta(planet,true),meta(engine(planet),true),meta(topology),meta(context,true),meta(magic)];
 return {
  program,magic,context,vault,delegation,topology,sponsor,planet,engine,
  starter: owner => planet(owner,0n),
  updateRules: (authority,planet) => ix(31,[meta(authority,false,true),...runtime(planet)]),
  migratePlanet: (authority,admin,planet) => ix(30,[meta(authority,false,true),meta(sponsor(admin),true),meta(planet,true),meta(vault,true),meta(magic)]),
  authorizeSession: (owner,session) => ix(27,[meta(owner,false,true),meta(planet(owner,0n),true)],session.toBytes()),
  createWithSession: (owner,admin,session,seed,nonce=0n) => {const p=planet(owner,nonce);return ix(28,[meta(owner),meta(admin,false,true),meta(session,false,true),meta(sponsor(admin),true),meta(p,true),meta(engine(p),true),meta(vault,true),meta(magic)],integer(nonce,8),integer(seed,4));},
  linkSession: (owner,admin,session) => ix(29,[meta(admin,false,true),meta(session,false,true),meta(sponsor(admin),true),meta(planet(owner,0n),true)]),
  initializeSponsor: (admin,funding) => ix(10,[meta(admin,true,true),meta(sponsor(admin),true),meta(SystemProgram.programId)],integer(funding,8)),
  delegateSponsor: (admin,validator) => {const s=sponsor(admin);return ix(11,[meta(admin,true,true),meta(s,true),meta(program),meta(pda([text('buffer'),s.toBytes()]),true),meta(pda([text('delegation'),s.toBytes()],delegation),true),meta(pda([text('delegation-metadata'),s.toBytes()],delegation),true),meta(delegation),meta(SystemProgram.programId)],validator.toBytes());},
  initializeAndDelegate(admin,funding,validator) {return [this.initializeSponsor(admin,funding),this.delegateSponsor(admin,validator)];},
  prepareTopology: admin => ix(12,[meta(admin,false,true),meta(sponsor(admin),true),meta(topology,true),meta(vault,true),meta(magic)]),
  createPlanet: (owner,admin,nonce,seed,session) => {const p=planet(owner,nonce);return ix(20,[meta(owner,false,true),meta(admin,false,true),meta(sponsor(admin),true),meta(p,true),meta(engine(p),true),meta(vault,true),meta(magic)],integer(nonce,8),integer(seed,4),session.toBytes());},
  build: (owner,planet,tile,kind,facing) => ix(21,action(owner,planet),integer(tile,4),integer(kind,1),integer(facing,1)),
  demolish: (owner,planet,tile) => ix(22,action(owner,planet),integer(tile,4)),
  setPaused: (owner,planet,tile,paused) => ix(23,action(owner,planet),integer(tile,4),integer(Number(paused),1)),
  advance: planet => ix(24,runtime(planet)),
  closePlanet: (owner,admin,planet) => ix(26,[meta(owner,false,true),meta(sponsor(admin),true),meta(planet,true),meta(engine(planet),true),meta(vault,true),meta(context,true),meta(magic)]),
  closeTopology: admin => ix(13,[meta(admin,false,true),meta(sponsor(admin),true),meta(topology,true),meta(vault,true),meta(magic)]),
  requestSponsorUndelegation: admin => ix(14,[meta(admin,true,true),meta(sponsor(admin),true),meta(context,true),meta(magic)]),
  reclaimSponsor: admin => ix(16,[meta(admin,true,true),meta(sponsor(admin),true)]),
 };
}
export function decodePlanet(data) {
 if (data.length !== layout.planetBytes && data.length !== layout.legacyPlanetBytes || !['IIWORLD1','IIWORLD2'].includes(new TextDecoder().decode(data.slice(0,8)))) throw Error('Invalid planet');
 const view = new DataView(data.buffer,data.byteOffset,data.byteLength), u64 = at => view.getBigUint64(at,true);
 return {rulesVersion:data[7]===50?2:1,needsVersion:data.length===layout.planetBytes?1:0,clothesFulfillment:data.length===layout.planetBytes?u64(6320):0n,beerFulfillment:data.length===layout.planetBytes?u64(6328):0n,radioCoveredHouses:data.length===layout.planetBytes?u64(6336):0n,generators:data.length===layout.planetBytes?u64(6344):0n,powerDemand:data.length===layout.planetBytes?u64(6352):0n,session:data.slice(6288,6320),owner:data.slice(8,40),sponsor:data.slice(40,72),nonce:u64(72),seed:view.getUint32(80,true),resolution:view.getUint32(84,true),tick:u64(344),next:u64(352),workers:u64(360),population:u64(368),houses:u64(1032),coveredHouses:u64(1048),foodFulfillment:u64(1056),growth:u64(1064),peakPopulation:u64(1112),phase:u64(1120),revision:u64(1128),scheduledAt:u64(1136),buildings:Array.from({length:642},(_,tile)=>{const at=1152+tile*8;return {tile,kind:data[at]-1,facing:data[at+1],paused:Boolean(data[at+2]),paid:data[at+3]};}).filter(b=>b.kind>=0)};
}
