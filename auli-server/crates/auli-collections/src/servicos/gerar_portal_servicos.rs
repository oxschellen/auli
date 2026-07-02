/// The service description without its leading `tipo / classe / titulo` header lines, which
/// `build_descricao` (in extrair_descricoes.rs / sc.rs) prepends. Those three fields become columns
/// of their own in the snapshot, so dropping the header here yields the clean body stored in
/// `ServicoRaw::descricao`. An empty/missing description yields an empty body.
pub(super) fn descricao_body(descricao: &str) -> String {
    descricao
        .lines()
        .skip(3)
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
