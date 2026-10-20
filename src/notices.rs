// notices.rs — Fila de avisos para o usuário vindos de fora da UI (BF-504).
//
// Persistência, integração com o Windows e outros pontos sem acesso ao app
// registram aqui o que deu errado; o loop da UI drena a fila e mostra toasts.
// Assim nenhuma falha acionável fica escondida atrás de `let _ =`.

use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub kind: NoticeKind,
    pub message: String,
}

/// Limite para a fila não crescer se a UI ficar muito tempo sem drenar.
const MAX_PENDING: usize = 16;

static PENDING: Mutex<Vec<Notice>> = Mutex::new(Vec::new());

fn push(kind: NoticeKind, message: String) {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if pending.len() >= MAX_PENDING {
        pending.remove(0);
    }
    pending.push(Notice { kind, message });
}

pub fn info(message: impl Into<String>) {
    push(NoticeKind::Info, message.into());
}

pub fn error(message: impl Into<String>) {
    push(NoticeKind::Error, message.into());
}

pub fn has_pending() -> bool {
    !PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty()
}

/// Retira o aviso pendente mais antigo. A UI chama quando não há toast
/// visível, então avisos simultâneos aparecem em sequência.
pub fn pop() -> Option<Notice> {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    (!pending.is_empty()).then(|| pending.remove(0))
}

/// Retira todos os avisos pendentes, do mais antigo ao mais novo.
#[cfg(test)]
pub fn drain() -> Vec<Notice> {
    let mut pending = PENDING
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    std::mem::take(&mut *pending)
}

#[cfg(test)]
pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notices_are_drained_in_order_and_bounded() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        drain();
        info("um");
        error("dois");
        let drained = drain();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained[0].kind, NoticeKind::Info);
        assert_eq!(drained[1].message, "dois");
        assert!(drain().is_empty());

        info("a");
        info("b");
        assert_eq!(pop().unwrap().message, "a");
        assert_eq!(pop().unwrap().message, "b");
        assert!(pop().is_none());

        for i in 0..(MAX_PENDING + 5) {
            info(format!("{i}"));
        }
        let drained = drain();
        assert_eq!(drained.len(), MAX_PENDING);
        assert_eq!(
            drained.last().unwrap().message,
            format!("{}", MAX_PENDING + 4)
        );
    }
}
