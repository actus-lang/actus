use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::{OnceLock, RwLock};

use super::types::DeclarationIdentity;

/// Interned canonical declaration identity (ADR-0108, Representation).
///
/// A handle is a reference to one immutable interned entry. Reading the
/// identity, its canonical key, equality, hashing, and ordering never take a
/// lock; only interning a new identity does. Entries live for the process,
/// and memory grows only with the number of distinct identities.
#[derive(Clone, Copy)]
pub struct DeclarationId(&'static InternedDeclaration);

struct InternedDeclaration {
    identity: DeclarationIdentity,
    key: Box<str>,
    stable_hash: u64,
}

type Interner = RwLock<HashMap<DeclarationIdentity, &'static InternedDeclaration>>;

fn interner() -> &'static Interner {
    static INTERNER: OnceLock<Interner> = OnceLock::new();
    INTERNER.get_or_init(|| RwLock::new(HashMap::new()))
}

impl DeclarationId {
    pub fn intern(identity: DeclarationIdentity) -> Self {
        if let Some(entry) =
            interner().read().expect("declaration interner poisoned").get(&identity)
        {
            return Self(entry);
        }
        let mut entries = interner().write().expect("declaration interner poisoned");
        if let Some(entry) = entries.get(&identity) {
            return Self(entry);
        }
        let key = identity.key().into_boxed_str();
        let stable_hash = stable_hash(&key);
        let entry: &'static InternedDeclaration = Box::leak(Box::new(InternedDeclaration {
            identity: identity.clone(),
            key,
            stable_hash,
        }));
        entries.insert(identity, entry);
        Self(entry)
    }

    pub fn identity(self) -> &'static DeclarationIdentity {
        &self.0.identity
    }

    /// Canonical key; ordering and every output use this, never the handle.
    pub fn canonical_key(self) -> &'static str {
        &self.0.key
    }

    pub fn key(self) -> String {
        self.0.key.to_string()
    }
}

/// FNV-1a over the canonical key: deterministic across runs and threads, so
/// hash-based containers never depend on interning order or addresses.
fn stable_hash(key: &str) -> u64 {
    key.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

impl PartialEq for DeclarationId {
    fn eq(&self, other: &Self) -> bool {
        // One identity has exactly one interned entry.
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for DeclarationId {}

impl Hash for DeclarationId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.stable_hash);
    }
}

impl Ord for DeclarationId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if std::ptr::eq(self.0, other.0) {
            return std::cmp::Ordering::Equal;
        }
        self.0.key.cmp(&other.0.key)
    }
}

impl PartialOrd for DeclarationId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Debug for DeclarationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("DeclarationId").field(&self.canonical_key()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{DeclarationId, DeclarationIdentity};

    fn builtin(name: &str) -> DeclarationIdentity {
        DeclarationIdentity::Builtin { name: name.to_owned() }
    }

    #[test]
    fn one_identity_has_one_handle() {
        let first = DeclarationId::intern(builtin("InternedOnce"));
        let second = DeclarationId::intern(builtin("InternedOnce"));
        assert_eq!(first, second);
        assert!(std::ptr::eq(first.identity(), second.identity()));
        assert_eq!(first.canonical_key(), "builtin::InternedOnce");
    }

    #[test]
    fn concurrent_interning_returns_one_handle() {
        let handles = std::thread::scope(|scope| {
            (0..8)
                .map(|_| scope.spawn(|| DeclarationId::intern(builtin("InternedConcurrently"))))
                .collect::<Vec<_>>()
                .into_iter()
                .map(|handle| handle.join().expect("interning thread"))
                .collect::<Vec<_>>()
        });
        assert!(handles.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    fn ordering_and_hash_follow_the_canonical_key() {
        let zed = DeclarationId::intern(builtin("OrderZed"));
        let alpha = DeclarationId::intern(builtin("OrderAlpha"));
        let mut handles = vec![zed, alpha];
        handles.sort();
        assert_eq!(handles, vec![alpha, zed]);
        let hash = |id: DeclarationId| {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            id.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(zed), hash(DeclarationId::intern(builtin("OrderZed"))));
        assert_eq!(format!("{zed:?}"), "DeclarationId(\"builtin::OrderZed\")");
    }
}
