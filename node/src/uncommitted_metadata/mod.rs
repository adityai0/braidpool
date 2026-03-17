use bitcoin::absolute::Time;
use bitcoin::consensus::encode::Decodable;
use bitcoin::consensus::encode::Encodable;
use bitcoin::consensus::encode::Error;
use bitcoin::io::{self, Read, Write};
use core::str::FromStr;
use rand::rngs::OsRng;
use serde::Deserialize;
use serde::Serialize;
use stratum_apps::secp256k1::schnorr::Signature;
use stratum_apps::secp256k1::Keypair;
use stratum_apps::secp256k1::Message;
use stratum_apps::secp256k1::Secp256k1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnCommittedMetadata {
    pub extra_nonce_1: Vec<u8>,
    pub extra_nonce_2: Vec<u8>,
    pub broadcast_timestamp: Time,
    pub signature: Signature,
}
impl Default for UnCommittedMetadata {
    fn default() -> Self {
        let secp = Secp256k1::new();
        let mut rng = OsRng::default();
        let (secret_key, _) = secp.generate_keypair(&mut rng);
        let msg = Message::from_digest([0u8; 32]);
        let default_sig = secp.sign_schnorr(&msg, &Keypair::from_secret_key(&secp, &secret_key));

        Self {
            extra_nonce_1: Vec::new(),
            extra_nonce_2: Vec::new(),
            broadcast_timestamp: Time::MIN,
            signature: default_sig,
        }
    }
}
impl Encodable for UnCommittedMetadata {
    fn consensus_encode<W: Write + ?Sized>(&self, w: &mut W) -> Result<usize, io::Error> {
        let mut len = 0;
        len += self.extra_nonce_1.consensus_encode(w)?;
        len += self.extra_nonce_2.consensus_encode(w)?;
        len += self
            .broadcast_timestamp
            .to_consensus_u32()
            .consensus_encode(w)?;
        len += self.signature.to_string().consensus_encode(w)?;
        Ok(len)
    }
}

impl Decodable for UnCommittedMetadata {
    fn consensus_decode<R: Read + ?Sized>(r: &mut R) -> Result<Self, Error> {
        let extra_nonce_1 = Vec::consensus_decode(r)?;
        let extra_nonce_2 = Vec::consensus_decode(r)?;
        let broadcast_timestamp = Time::from_consensus(u32::consensus_decode(r).unwrap()).unwrap();
        let signature = Signature::from_str(&String::consensus_decode(r).unwrap()).unwrap();

        Ok(UnCommittedMetadata {
            extra_nonce_1,
            extra_nonce_2,
            broadcast_timestamp,
            signature,
        })
    }
}
