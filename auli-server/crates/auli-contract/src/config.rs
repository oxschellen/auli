//! `config/` — **onde o catálogo mora**, e a única aritmética de caminho que o resolve.
//!
//! O `registry.toml` e os `prompts/` não são dado coletado: são **configuração**. Descrevem quais
//! entidades existem, como se chamam e com que instruções o LLM responde por cada uma. Morar dentro
//! de `data/` comunicava o contrário — que fossem mais um artefato do pipeline —, e escondia de
//! quem lê o repo o ponto por onde se adapta o Auli para outra organização.
//!
//! ```text
//! config/                 ← catálogo (versionado): quem existe e como responde
//!   registry.toml
//!   prompts/
//! data/                   ← dado (fora do git, exceto o que o scraper produz)
//!   <id>/{raw,ref,docs,packs}/
//! ```
//!
//! **A resolução é por convenção, num lugar só.** `config/` é IRMÃ da raiz de dados, então
//! [`config_dir`] a deriva do `data_dir` que cada binário já conhece; `AULI_CONFIG_DIR` sobrepõe,
//! para quem servir de outro lugar. Espalhar esse cálculo é como as duas raízes divergem em
//! silêncio — daí este módulo ser o único a fazê-lo.

use std::path::{Path, PathBuf};

/// O diretório de configuração: `$AULI_CONFIG_DIR`, ou a irmã `config/` da raiz de dados.
///
/// `data_dir` é relativo ao CWD de quem chama (`./data` no servidor, `../data` no
/// `auli-collections`), e a irmã sai por `parent()` — `../data` ⇒ `../config`. Raiz sem componente
/// de pai (`"data"`) cai no diretório corrente, que é o mesmo lugar.
pub fn config_dir(data_dir: &Path) -> PathBuf {
    if let Ok(p) = std::env::var("AULI_CONFIG_DIR")
        && !p.trim().is_empty()
    {
        return PathBuf::from(p);
    }
    data_dir
        .parent()
        .unwrap_or(Path::new("."))
        .join(NOME_CONFIG)
}

const NOME_CONFIG: &str = "config";
const NOME_REGISTRY: &str = "registry.toml";

/// Caminho do registro de entidades.
pub fn registry_path(data_dir: &Path) -> PathBuf {
    config_dir(data_dir).join(NOME_REGISTRY)
}

/// Caminho de um prompt, a partir do valor GRAVADO no registry (ex.: `"prompts/rs.txt"`).
///
/// Aqueles valores não mudaram na migração: sempre foram relativos à pasta do registry, e
/// continuam sendo. É por isso que mover a pasta não exigiu tocar no conteúdo dela.
pub fn prompt_path(data_dir: &Path, relativo: &str) -> PathBuf {
    config_dir(data_dir).join(relativo)
}

/// Recusa o layout ANTIGO, em que o catálogo vivia dentro de `data/`.
///
/// `Err` quando existe um `data/registry.toml` — mesmo que o novo também exista. Ler o novo e
/// ignorar o velho em silêncio é o modo de falha caro: alguém edita o arquivo errado e passa uma
/// tarde sem entender por que a mudança não pega. Ambiente de execução é um só (D-CFG-6), então a
/// migração é atômica e um resíduo do layout antigo é sempre um erro a corrigir, não um estado a
/// tolerar.
pub fn recusar_layout_antigo(data_dir: &Path) -> Result<(), String> {
    let legado = data_dir.join(NOME_REGISTRY);
    if !legado.exists() {
        return Ok(());
    }
    Err(format!(
        "`{}` é do layout ANTIGO — o catálogo mudou para `{}`.\n\
         Mova o que ainda estiver lá e apague o resíduo:\n  \
         git mv {} {}\n  \
         git mv {}/prompts {}/prompts\n\
         Nenhum arquivo é lido do caminho antigo: dois catálogos vivos é como uma edição some sem \
         explicação.",
        legado.display(),
        config_dir(data_dir).display(),
        legado.display(),
        registry_path(data_dir).display(),
        data_dir.display(),
        config_dir(data_dir).display(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As variáveis de ambiente são globais ao processo, e `cargo test` roda em threads. Este mutex
    /// serializa os testes que mexem em `AULI_CONFIG_DIR` — sem ele, um teste apaga a var enquanto
    /// o outro a lê, e a falha é intermitente (a pior de diagnosticar).
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn config_e_irma_da_raiz_de_dados() {
        let _g = ENV.lock().unwrap();
        unsafe { std::env::remove_var("AULI_CONFIG_DIR") };
        // Os dois CWDs reais do projeto: o servidor roda em `auli-server/` (`../data`), o
        // `auli-collections` idem; o default do binário é `./data`.
        assert_eq!(config_dir(Path::new("../data")), PathBuf::from("../config"));
        assert_eq!(config_dir(Path::new("./data")), PathBuf::from("./config"));
        assert_eq!(
            config_dir(Path::new("/srv/auli/data")),
            PathBuf::from("/srv/auli/config")
        );
        // Raiz sem componente de pai: o irmão é o diretório corrente.
        assert_eq!(config_dir(Path::new("data")), PathBuf::from("config"));
    }

    #[test]
    fn a_env_var_sobrepoe() {
        let _g = ENV.lock().unwrap();
        unsafe { std::env::set_var("AULI_CONFIG_DIR", "/outro/lugar") };
        assert_eq!(
            config_dir(Path::new("../data")),
            PathBuf::from("/outro/lugar")
        );
        assert_eq!(
            registry_path(Path::new("../data")),
            PathBuf::from("/outro/lugar/registry.toml")
        );
        // Vazia é como "não definida" — um `export AULI_CONFIG_DIR=` num script não pode mandar o
        // servidor procurar o catálogo na raiz do sistema de arquivos.
        unsafe { std::env::set_var("AULI_CONFIG_DIR", "  ") };
        assert_eq!(config_dir(Path::new("../data")), PathBuf::from("../config"));
        unsafe { std::env::remove_var("AULI_CONFIG_DIR") };
    }

    #[test]
    fn o_prompt_e_relativo_a_pasta_do_registry() {
        let _g = ENV.lock().unwrap();
        unsafe { std::env::remove_var("AULI_CONFIG_DIR") };
        // O valor vem do registry SEM alteração: `prompt = "prompts/rs.txt"` desde antes da
        // migração. Se este teste falhar, o registry precisou ser reescrito — e não precisava.
        assert_eq!(
            prompt_path(Path::new("../data"), "prompts/rs.txt"),
            PathBuf::from("../config/prompts/rs.txt")
        );
    }

    #[test]
    fn o_layout_antigo_e_recusado_com_o_comando_da_migracao() {
        let base = std::env::temp_dir().join(format!("auli-cfg-legado-{}", std::process::id()));
        let data = base.join("data");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&data).unwrap();

        // Sem resíduo: passa.
        assert!(recusar_layout_antigo(&data).is_ok());

        // Com `data/registry.toml`: erro que NOMEIA os dois caminhos e dá o comando.
        std::fs::write(data.join("registry.toml"), "").unwrap();
        let e = recusar_layout_antigo(&data).unwrap_err();
        assert!(e.contains("layout ANTIGO"), "{e}");
        assert!(e.contains("git mv"), "tem de dar o comando: {e}");
        assert!(
            e.contains(&data.display().to_string()),
            "cita o antigo: {e}"
        );
        assert!(
            e.contains(&base.join("config").display().to_string()),
            "cita o novo: {e}"
        );
        let _ = std::fs::remove_dir_all(&base);
    }
}
