angular.module('portalCidadaoApp.templates', ['components/campanha/views/como-funciona.html', 'components/campanha/views/eventos.html', 'components/campanha/views/futebol.html', 'components/campanha/views/ingressos.html', 'components/campanha/views/instituicoes.html', 'components/campanha/views/ipva.html', 'components/campanha/views/legislacao.html', 'components/campanha/views/regras-do-sorteio.html', 'components/campanha/views/sobre.html', 'components/campanha/views/sorteios.html', 'components/campanha/views/termo-de-uso.html', 'components/campanha/views/vouchers-do-turismo.html', 'components/comunicacao/views/assessoria-comunicacao.html', 'components/comunicacao/views/clipping.html', 'components/comunicacao/views/contato.html', 'components/comunicacao/views/faq.html', 'components/comunicacao/views/informativos.html', 'components/comunicacao/views/legislacao.html', 'components/comunicacao/views/manuais.html', 'components/comunicacao/views/material-grafico.html', 'components/comunicacao/views/noticia-detalhe.html', 'components/comunicacao/views/noticias.html', 'components/contato/views/contato.html', 'components/educacao-fiscal/views/material.html', 'components/educacao-fiscal/views/origem.html', 'components/educacao-fiscal/views/sobre.html', 'components/enquetes/views/enquete.html', 'components/futebol/views/como-funciona.html', 'components/futebol/views/escolher-time.html', 'components/futebol/views/futebol-preferidos.html', 'components/futebol/views/futebol-valores.html', 'components/futebol/views/rateio-times.html', 'components/futebol/views/times-cadastrados.html', 'components/home/views/appdown.html', 'components/home/views/evento-detalhe.html', 'components/home/views/home.html', 'components/instituicoes/views/beneficiadas.html', 'components/instituicoes/views/como-funciona.html', 'components/instituicoes/views/instituicoes-cadastradas.html', 'components/instituicoes/views/pontuacao.html', 'components/instituicoes/views/premiacoes.html', 'components/instituicoes/views/troca-da-instituicao-favorita.html', 'components/oaplicativo/views/baixar-app.html', 'components/oaplicativo/views/faq.html', 'components/oaplicativo/views/manuais.html', 'components/oaplicativo/views/versoes.html', 'components/sorteios/views/bilhetes.html', 'components/sorteios/views/calendario-de-premios.html', 'components/sorteios/views/como-funciona.html', 'components/sorteios/views/desconto-ipva-informacoes.html', 'components/sorteios/views/descontos-ipva.html', 'components/sorteios/views/detalhes-dos-sorteios.html', 'components/sorteios/views/exibicao-dos-premiados.html', 'components/sorteios/views/ganhadores.html', 'components/sorteios/views/premiacao-informacoes.html', 'components/sorteios/views/premiacao-valores.html', 'components/sorteios/views/proximo-sorteio.html', 'components/sorteios/views/sorteio-calculo-bilhete-ate-julho-2020.html', 'components/sorteios/views/sorteio-calculo-bilhete.html', 'shared/breadcrumb/views/breadcrumb.html', 'shared/messages/views/error.html', 'shared/modal/views/modal.html', 'shared/rodape/views/rodapeDirective.html', 'shared/topo/views/topoDirective.html', 'layouts/layout-home.html', 'layouts/layout-interna-bg.html', 'layouts/layout-interna.html', 'components/campanha/views/como-participar/cidadaos.html', 'components/campanha/views/como-participar/empresarios.html', 'components/campanha/views/como-participar/instituicoes.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_1_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_2_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_3_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_4_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_5_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_6_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_7_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_8_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_9_sorteio.html', 'components/sorteios/views/ganhadores-sorteios/ganhadores_sorteio.html']);

angular.module("components/comunicacao/views/contato.html", []).run(["$templateCache", function ($templateCache) {
  $templateCache.put("components/comunicacao/views/contato.html",
    "<section id=\"inner-headline\">\n" +
    "    <div class=\"container\">\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span8\">\n" +
    "                <div class=\"inner-heading\">\n" +
    "                    <h2>Comunicação</h2>\n" +
    "                </div>\n" +
    "            </div>\n" +
    "            <div class=\"span4\">\n" +
    "                <!-- <div ncy-breadcrumb></div> -->\n" +
    "                <ul class=\"breadcrumb\">\n" +
    "                    <li><a ui-sref=\"home\"><i class=\"icon-home\"></i></a><i class=\"icon-angle-right\"></i></li>\n" +
    "                    <li>Comunicação</i></a><i class=\"icon-angle-right\"></i></li>\n" +
    "                    <li class=\"active\">Contato</li>\n" +
    "                </ul>\n" +
    "            </div>\n" +
    "        </div>\n" +
    "    </div>\n" +
    "</section>\n" +
    "\n" +
    "\n" +
    "<section id=\"content\">\n" +
    "    <div class=\"container\">\n" +
    "        <h1 class=\"titulo-pagina-h1\">Contato</h1>\n" +
    "        <!--\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span6\">\n" +
    "                <p>Deseja entrar em contato conosco?</p>\n" +
    "                <p> Use nosso atendimento: telefônico: <b>(84) 3209-7880</b> (seg à sex, de 8h às 14h)</p>\n" +
    "                <p>E-mail: atendimento@sefaz.rn.gov.br</p>\n" +
    "                <p>Whatsapp: <b><a href=\"https://web.whatsapp.com/send?phone=558432327077&text=Pode%20me%20ajudar?\">clique aqui</a></b></p>\n" +
    "            </div>\n" +
    "            <div class=\"span6\">\n" +
    "                    <h1 class=\"titulo-pagina-h1\">Nossas redes sociais</h1>\n" +
    "                    <p>Que tal nos seguir e ficar por dentro das novidades?</p>\n" +
    "                    <ul class=\"social-network\">\n" +
    "                    <li><a href=\"https://www.facebook.com/NotaPotiguar\" target=\"_blank\" data-placement=\"bottom\" title=\"Facebook\"><i class=\"fab fa-facebook footer-icons\"></i> &nbsp; Facebook</a></li>\n" +
    "                    <li><a href=\"https://www.instagram.com/NotaPotiguar\" target=\"_blank\" data-placement=\"bottom\" title=\"Instagram\"><i class=\"fab fa-instagram footer-icons\"></i>  &nbsp;  Instagram</a></li>\n" +
    "                    <li><a href=\"https://twitter.com/NotaPotiguar\" target=\"_blank\" data-placement=\"bottom\" title=\"Twitter\"><i class=\"fab fa-twitter footer-icons\"></i>  &nbsp;  Twitter</a></li>\n" +
    "                </ul>\n" +
    "            </div>\n" +
    "        </div>\n" +
    "        -->\n" +
    "\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span6\">\n" +
    "                <form name=\"faleconosco\" novalidate=\"\" class=\"form\">\n" +
    "                    <div class=\"row\">\n" +
    "                        <div class=\"span6\">\n" +
    "                            <div class=\"form-group m-t-3\">\n" +
    "                                <input class=\"form-control\" id=\"Nome\" name=\"Nome\" type=\"text\" required=\"required\" ng-model=\"model.nome\">\n" +
    "                                <label for=\"nome\" class=\"form-control-placeholder\">Nome</label>\n" +
    "                            </div>\n" +
    "                        </div>\n" +
    "                    </div>\n" +
    "\n" +
    "                    <div class=\"row\">\n" +
    "                            <div class=\"span6\">\n" +
    "                            <div class=\"form-group m-t-3\">\n" +
    "                                <input class=\"form-control\" id=\"Email\" name=\"Email\" type=\"text\" required=\"required\" ng-model=\"model.email\">\n" +
    "                                <label for=\"email\" class=\"form-control-placeholder\">E-mail</label>\n" +
    "                            </div>\n" +
    "                        </div>\n" +
    "                    </div>\n" +
    "\n" +
    "                    <div class=\"row\">\n" +
    "                        <div class=\"span1\">\n" +
    "                            <div class=\"form-group\">\n" +
    "                                <input type=\"tel\" ng-model=\"instituicao.telefoneDdd\" name=\"telefoneDdd\"\n" +
    "                                    id=\"telefoneDdd\" class=\"form-control span2\" maxlength=\"2\" required=\"required\"\n" +
    "                                    ui-mask=\"99\" ui-mask-placeholder ui-options=\"{clearOnBlur: true}\"\n" +
    "                                    ui-mask-placeholder-char=\"space\" />\n" +
    "                                \n" +
    "                                <label for=\"telefoneDdd\" class=\"form-control-placeholder\">DDD *</label>\n" +
    "                            </div>\n" +
    "                        </div>\n" +
    "                        <div class=\"span5\">\n" +
    "                            <div class=\"form-group\">\n" +
    "                                <input type=\"tel\" ng-model=\"instituicao.telefone\" name=\"telefone\" id=\"telefone\"\n" +
    "                                    class=\"form-control span4\" maxlength=\"9\" required=\"required\" ui-mask=\"99999999?9?\"\n" +
    "                                    ui-mask-placeholder ui-options=\"{clearOnBlur: true}\"\n" +
    "                                    ui-mask-placeholder-char=\"space\" />\n" +
    "                                \n" +
    "                                <label for=\"telefone\" class=\"form-control-placeholder\">Telefone *</label>\n" +
    "                            </div>\n" +
    "                        </div>\n" +
    "                    </div>\n" +
    "                    \n" +
    "                    <div class=\"row\">\n" +
    "                        <div class=\"span6\">\n" +
    "                            <div class=\"form-group m-t-3\">\n" +
    "                                <textarea class=\"form-control\" id=\"Mensagem\" name=\"Mensagem\" rows=\"4\" required=\"required\" ng-model=\"model.mensagem\"></textarea>\n" +
    "                                <label for=\"Mensagem\" class=\"form-control-placeholder\">Mensagem</label>\n" +
    "                            </div>\n" +
    "                        </div>\n" +
    "                    </div>\n" +
    "                    <div class=\"row\">\n" +
    "                        <div class=\"span6\">\n" +
    "                            <div class=\"form-group m-t-3\">\n" +
    "                                <input type=\"button\" value=\"Enviar\" class=\"btn btn-large btn-theme\" ng-disabled=\"faleconosco.$invalid\" ng-click=\"enviarMensagem()\" disabled=\"disabled\">\n" +
    "                            </div>\n" +
    "                        </div>\n" +
    "                    </div>\n" +
    "                    \n" +
    "                    \n" +
    "                </form>\n" +
    "            </div>\n" +
    "\n" +
    "\n" +
    "            <div class=\"span6\">\n" +
    "                <h3>Canais de atendimento:</h3>\n" +
    "                <p>Telefone: <b>(84) 3209-7880 (seg à sex, de 8h às 14h)</b></p>\n" +
    "                <p>E-mail: <b>atendimento@sefaz.rn.gov.br</b></p>\n" +
    "            </div>\n" +
    "        </div>\n" +
    "    </div>\n" +
    "</section>");
}]);

angular.module("components/comunicacao/views/faq.html", []).run(["$templateCache", function ($templateCache) {
  $templateCache.put("components/comunicacao/views/faq.html",
    "<section id=\"inner-headline\">\n" +
    "    <div class=\"container\">\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span8\">\n" +
    "                <div class=\"inner-heading\">\n" +
    "                    <h2>Comunicação</h2>\n" +
    "                </div>\n" +
    "            </div>\n" +
    "            <div class=\"span4\">\n" +
    "                <!-- <div ncy-breadcrumb></div> -->\n" +
    "                <ul class=\"breadcrumb\">\n" +
    "                    <li><a ui-sref=\"home\"><i class=\"icon-home\"></i></a><i class=\"icon-angle-right\"></i></li>\n" +
    "                    <li>Comunicação</a><i class=\"icon-angle-right\"></i></li>\n" +
    "                    <li class=\"active\">FAQ</li>\n" +
    "                </ul>\n" +
    "            </div>\n" +
    "        </div>\n" +
    "    </div>\n" +
    "</section>\n" +
    "\n" +
    "\n" +
    "<section id=\"content\">\n" +
    "    <div class=\"container\">\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span12\">\n" +
    "                <h1 class=\"titulo-pagina-h1\">Perguntas Frequentes</h1>\n" +
    "\n" +
    "                <div class=\"FAQSide\">\n" +
    "\n" +
    "                        <h3 class=\"titulo-pagina-h3\">O que é o aplicativo \"Nota Potiguar\"?</h3>\n" +
    "                        <p>\n" +
    "                        É um aplicativo de estímulo à cidadania fiscal do Estado do Rio Grande do Norte, que tem por objetivo incentivar os consumidores a exigirem o documento fiscal. Com ele, você poderá consultar notas por período, ver relatórios de suas despesas, reclamar de algum estabelecimento quando presenciar algo errado, entre outras ferramentas da Tributação Estadual.\n" +
    "                        </p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Como posso participar?</h3>\n" +
    "                        <p>Baixando o aplicativo \"Nota Potiguar\", que está disponível nas versões para Android e iOS. Cadastre seu CPF no momento da compra em qualquer estabelecimento comercial. </p>\n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Qual a importância do aplicativo?</h3>\n" +
    "                        <p>Incentivar o consumidor, pessoa física, a solicitar a Nota Fiscal do Consumidor Eletrônica - NFC-e ou Nota Fiscal Eletrônica - NF-e <br>\n" +
    "                        Contribuir para o fortalecimento do exercício da cidadania e a disseminação da função socioeconômica dos tributos com ações de Educação Fiscal.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Quais os benefícios?</h3>\n" +
    "                        <p>Facilidade na identificação de seus documentos fiscais pela disponibilização de extrato eletrônico de suas compras;<br>\n" +
    "                        Estímulo educação financeira, com a disponibilidade de gráficos que apresentam relatórios de suas despesas;<br>\n" +
    "                        Oportunidade de exercer a cidadania fiscal;<br>\n" +
    "                        Possiblidade de impressão de cópia do documento fiscal, a qualquer tempo, dispensando assim a guarda para comprovação da garantia da mercadoria ou produto.<br>\n" +
    "                        </p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Preciso me cadastrar no aplicativo antes de realizar minhas compras?</h3>\n" +
    "                        <p>Sim. É necessário o cadastro prévio para que suas compras sejam reconhecidas pelo aplicativo.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">A empresa não emitiu a nota. Como devo proceder? </h3>\n" +
    "                        <p>O cidadão poderá efetuar uma reclamação através do aplicativo, na item de menu \"Reclamações\", ou através do site da Campanha Nota Potiguar (appnp.sefaz.rn.gov.br).</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Existe prazo máximo para ser realizada uma reclamação?</h3>\n" +
    "                        <p>Não. Porém, a reclamação deve ser feita o mais rápido possível para que a fiscalização da Secretaria de Tributação possa ter maior agilidade.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">A nota fiscal não aparece em minha conta no aplicativo. O que devo fazer?</h3>\n" +
    "                        <p>Após passarem 24 horas do período de emissão, o cidadão poderá através do QR Code, presente na nota fiscal, inseri-la no aplicativo. Caso o problema persista, faça uma reclamação através do aplicativo \"Nota Potiguar\" ou pelo site (appnp.sefaz.rn.gov.br)</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">A empresa registrou outro CPF em minha nota fiscal. Como devo proceder?</h3>\n" +
    "                        <p>Você pode solicitar à empresa que faça o cancelamento dessa nota e emita outra corretamente, com o seu CPF. Caso a empresa se negue a corrigir o erro, você poderá encaminhar uma reclamação à Secretária de Tributação, através do aplicativo ou do site da Campanha Nota Potiguar (appnp.sefaz.rn.gov.br).</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Quais empresas participantes?</h3>\n" +
    "                        <p>Todas as empresas estão aptas para o cadastramento do CPF nas notas fiscais. Entretanto os microempreendedores individuais (MEI) não são obrigados a emitir Nota Fiscal Eletrônica.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Ao consultar minha nota fiscal, foi informado que ela foi emitida em contingência. O que isso significa?</h3>\n" +
    "                        <p>Quer dizer que no momento da emissão do documento fiscal ocorreu algum problema técnico que impediu a transmissão da nota fiscal à SET/RN e que a empresa terá o prazo de até 24 horas para transmiti-la.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Posso informar meu CPF depois de emitida a nota fiscal eletrônica?</h3>\n" +
    "                        <p>Não. O CPF deve sempre ser informado no momento da compra.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Preciso informar meu nome, endereço e telefone às lojas para participar?</h3>\n" +
    "                        <p>Não. Basta informar apenas o seu CPF.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Posso alterar meus dados cadastrados?</h3>\n" +
    "                        <p>Sim. A qualquer momento os dados cadastrais podem ser atualizados no app \"Nota Potiguar\", porém nome, CPF e o nome da mãe não poderão ser alterados.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Como posso alterar minha senha?</h3>\n" +
    "                        <p>Na aba \"alterar sua conta\", você poderá trocar sua senha atual por outra a sua escolha, seguindo as especificações solicitadas pelo aplicativo.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Como posso recuperar minha senha?</h3>\n" +
    "                        <p>Ao entrar no aplicativo, o usuário terá a opção \"Redefinir senha\", nele você informará seu CPF e escolherá por qual meio deseja recuperá-la (SMS ou email). Ao final, receberá instruções para resgatar sua senha através do mecanismo elegido. </p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Qual a diferença entre Cupom Fiscal e Nota Fiscal?</h3>\n" +
    "                        <p>\n" +
    "                        <b>Cupom fiscal</b><br>\n" +
    "                        Documento emitido pela caixa registradora, ele disponibilizava poucas informações e geralmente quase nenhum dado sobre o cliente. Desde o dia 31/12/2017 os Emissores de Cupom Fiscal (ECF’s) dos comércios varejistas do Rio Grande do Norte cessaram o funcionamento. Em caso de uso indevido, o estabelecimento comercial está passível de multa pelo funcionamento fora das regras.<br>\n" +
    "                        <b>Nota fiscal</b><br>\n" +
    "                        Existem dois modelos de nota fiscal: a Nota Fiscal do Consumidor Eletrônica (NFC-e) e a Nota Fiscal Eletrônica (NF-e).  A NFC-e é direcionada ao consumidor e vem para substituir o cupom fiscal, já a NF-e atende a todas as outras situações possíveis, desde as operações de compra e venda, até as operações de devolução ou transferência de mercadorias, exportação, etc.\n" +
    "                        A Nota Fiscal é um instrumento mais eficiente para evitar fraudes fiscais, garantir que os direitos do consumidor sejam respeitados, além de ser o documento apropriado para solicitar a garantia dos produtos adquiridos.\n" +
    "                        </p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">A empresa pode exigir o cadastro de meu nome, endereço, telefone ou outros dados, para inserir meu CPF na nota fiscal eletrônica?</h3>\n" +
    "                        <p>A inclusão do número do CPF no documento fiscal eletrônico não pode ser condicionada a nenhuma espécie de cadastro prévio do consumidor no estabelecimento comercial. </p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Preciso guardar as notas fiscais impressas?</h3>\n" +
    "                        <p>Não é necessário, desde que a nota esteja no aplicativo. Após a sua inclusão, as notas fiscais ficam armazenadas, podendo ser acessadas quando o usuário preferir.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">A empresa pode estipular um valor mínimo de compra para inserir meu CPF na nota fiscal eletrônica?</h3>\n" +
    "                        <p>Não. Para qualquer venda, independentemente do valor, deverá ser emitido o documento fiscal.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Minhas notas fiscais serão informadas à Receita Federal?</h3>\n" +
    "                        <p>Não há nenhum acordo de transferência de informações entre a Secretaria Estadual de Tributação e a Receita Federal.</p>\n" +
    "                        \n" +
    "                        \n" +
    "                        \n" +
    "                        <h3 class=\"titulo-pagina-h3\">Como faço para baixar o aplicativo?</h3>\n" +
    "                        <p>\n" +
    "                        I. Na loja Google Play (Android) ou App Store (Apple) pesquise o aplicativo \"Nota Potiguar\" (ou baixe através do site appnp.sefaz.rn.gov.br)<br>\n" +
    "                        II. Clique em \"instalar\"<br>\n" +
    "                        III. Após instalado, clique no ícone do aplicativo no seu celular<br>\n" +
    "                        IV. Ao abrir o aplicativo clique em \"Faça o seu cadastro\". Informe todos os seus dados corretamente.<br>\n" +
    "                        V. Será enviado um e-mail para o endereço informado para confirmação de cadastro<br>\n" +
    "                        VI. Ao confirmar o cadastro, o aplicativo pode ser aberto novamente para o uso\n" +
    "                        </p>\n" +
    "                        \n" +
    "                \n" +
    "\n" +
    "\n" +
    "\n" +
    "                    </div>\n" +
    "            </div>\n" +
    "        </div>\n" +
    "    </div>\n" +
    "</section>");
}]);

angular.module("components/oaplicativo/views/baixar-app.html", []).run(["$templateCache", function ($templateCache) {
  $templateCache.put("components/oaplicativo/views/baixar-app.html",
    "<section id=\"inner-headline\">\n" +
    "    <div class=\"container\">\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span12\">\n" +
    "                <div class=\"inner-heading\">\n" +
    "                    <h2 class=\"cor-preta\">Baixar o Aplicativo Nota Potiguar</h2>\n" +
    "                    <br/>\n" +
    "                </div>\n" +
    "            </div>\n" +
    "            <div class=\"span1\"></div>\n" +
    "            <div class=\"span10\">\n" +
    "                <p align=\"center\"><img src=\"https://np.sefaz.rn.gov.br/portal/img/topo-logo.jpg\" alt=\"Nota Potiguar\"></p>\n" +
    "                \n" +
    "                <br/>\n" +
    "                <p align=\"center\"><b>Participar do Programa Nota Potiguar é simples: peça sua nota fiscal com CPF em compras no RN, acumule pontos, troque por prêmios e concorra a sorteios mensais. Faça parte da cidadania fiscal e ajude instituições sociais!</b></p>\n" +
    "                <br/>\n" +
    "                \n" +
    "                  <p align=\"center\">\n" +
    "                    <a href=\"https://apps.apple.com/br/app/nota-potiguar/id1320051838\" target=\"_blank\">\n" +
    "                      <img src=\"https://developer.apple.com/assets/elements/badges/download-on-the-app-store.svg\" alt=\"Baixar na App Store\" style=\"height: 55px\">\n" +
    "                    </a>\n" +
    "                    &nbsp;&nbsp;&nbsp;\n" +
    "                    <a href=\"https://play.google.com/store/apps/details?id=br.gov.rn.set.notafiscalpotiguar.app\" target=\"_blank\">\n" +
    "                      <img src=\"https://upload.wikimedia.org/wikipedia/commons/7/78/Google_Play_Store_badge_EN.svg\" alt=\"Baixar no Google Play\">\n" +
    "                    </a>\n" +
    "                </p>\n" +
    "            </div>\n" +
    "            <div class=\"span1\"></div>\n" +
    "        </div>\n" +
    "    </div>\n" +
    "</section>\n" +
    "");
}]);

angular.module("components/oaplicativo/views/faq.html", []).run(["$templateCache", function ($templateCache) {
  $templateCache.put("components/oaplicativo/views/faq.html",
    "<section id=\"inner-headline\">\n" +
    "    <div class=\"container\">\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span8\">\n" +
    "                <div class=\"inner-heading\">\n" +
    "                    <h2>O Aplicativo</h2>\n" +
    "                </div>\n" +
    "            </div>\n" +
    "            <div class=\"span4\">\n" +
    "                <!-- <div ncy-breadcrumb></div> -->\n" +
    "                <ul class=\"breadcrumb\">\n" +
    "                    <li><a ui-sref=\"home\"><i class=\"icon-home\"></i></a><i class=\"icon-angle-right\"></i></li>\n" +
    "                    <li>O Aplicativo</a><i class=\"icon-angle-right\"></i></li>\n" +
    "                    <li class=\"active\">FAQ</li>\n" +
    "                </ul>\n" +
    "            </div>\n" +
    "        </div>\n" +
    "    </div>\n" +
    "</section>\n" +
    "\n" +
    "\n" +
    "<section id=\"content\">\n" +
    "    <div class=\"container\">\n" +
    "        <div class=\"row\">\n" +
    "            <div class=\"span12\">\n" +
    "                <h1 class=\"titulo-pagina-h1\">FAQ (Perguntas Frequentes)</h1>\n" +
    "\n" +
    "                <h3 class=\"titulo-pagina-h3\">O que é o aplicativo \"Nota Potiguar\"?</h3>\n" +
    "                <p>\n" +
    "                    É um aplicativo de estímulo à cidadania fiscal do Estado do Rio Grande do Norte,\n" +
    "                    que tem\n" +
    "                    por objetivo incentivar os consumidores a exigirem o documento fiscal. Com ele,\n" +
    "                    você poderá\n" +
    "                    consultar notas por período, ver relatórios de suas despesas, denunciar algum\n" +
    "                    estabelecimento quando presenciar algo errado, entre outras ferramentas da\n" +
    "                    Tributação\n" +
    "                    Estadual.\n" +
    "                    \n" +
    "                </p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Como posso participar?</h3>\n" +
    "                <p>\n" +
    "                        Baixando o aplicativo \"Nota Potiguar\", que está\n" +
    "                        disponível\n" +
    "                        nas versões para Android e iOS. Cadastre seu CPF no momento da compra em qualquer\n" +
    "                        estabelecimento comercial.\n" +
    "                </p>\n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Qual a importância do aplicativo?</h3>\n" +
    "                <p>Incentivar o consumidor, pessoa física, a solicitar a Nota\n" +
    "                        Fiscal\n" +
    "                        do Consumidor Eletrônica - NFC-e ou Nota Fiscal Eletrônica - NF-e <br />\n" +
    "                        Contribuir para o fortalecimento do exercício da cidadania e a disseminação da\n" +
    "                        função\n" +
    "                        socioeconômica dos tributos com ações de Educação Fiscal.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Quais os benefícios?</h3>\n" +
    "                <p>Facilidade na identificação de seus documentos fiscais pela\n" +
    "                        disponibilização de extrato eletrônico de suas compras;<br />\n" +
    "                        Estímulo à educação financeira, com a disponibilidade de gráficos que apresentam\n" +
    "                        relatórios\n" +
    "                        de suas despesas;<br />\n" +
    "                        Oportunidade de exercer a cidadania fiscal;<br />\n" +
    "                        Possiblidade de impressão de cópia do documento fiscal, a qualquer tempo,\n" +
    "                        dispensando assim\n" +
    "                        a guarda para comprovação da garantia da mercadoria ou produto.<br /></p>\n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Como funciona o desconto no IPVA?</h3>\n" +
    "                <p>Cada ponto trocado corresponde a cerca de <b>0,1%</b> de desconto no IPVA de um veículo\n" +
    "                    de sua propriedade elegível para receber o benefício do programa,\n" +
    "                    limitando-se a <b>2%</b> acumulados ao mês até de novembro de 2019, e <b>1%</b> acumulado ao mês a partir de\n" +
    "                    dezembro de 2019, nunca ultrapassando o desconto máximo aplicável de <b>10%</b> ao ano para o veículo selecionado.\n" +
    "                    O período de acumulação vai até novembro do ano anterior ao do que se destinará o desconto no IPVA.\n" +
    "                    O mês de dezembro já contará para o próximo período de acumulação.\n" +
    "                    Você poderá selecionar o veículo que obterá o desconto até o dia 5 de dezembro de cada ano. Após esta data o desconto\n" +
    "                    não poderá ser transferido do veículo escolhido. </p>\n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Preciso me cadastrar no aplicativo antes de realizar minhas compras?</h3>\n" +
    "                <p>Sim. É necessário o cadastro prévio para que suas compras\n" +
    "                        sejam reconhecidas pelo aplicativo.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">A empresa não emitiu a nota. Como devo proceder?</h3>\n" +
    "                <p>O cidadão poderá efetuar uma reclamação através do\n" +
    "                        aplicativo, no item de menu\n" +
    "                        \"Reclamações\", ou através do site da Campanha Nota Potiguar (appnp.sefaz.rn.gov.br).</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Existe prazo máximo para ser realizada uma reclamação?</h3>\n" +
    "                <p>Não. Porém a reclamação deve ser feita o mais rapidamente\n" +
    "                        possível para\n" +
    "                        que a fiscalização da Secretaria de Estado da Tributação/RN seja eficaz.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">A nota fiscal não aparece em minha conta no aplicativo. O que devo fazer?</h3>\n" +
    "                <p>O cidadão poderá, através do QR Code, ou da chave numérica presente na nota fiscal, inseri-la no aplicativo.\n" +
    "                        Se o aplicativo indicar que a nota fiscal não foi carregada na base de dados da Tributação Estadual decorridas \n" +
    "                        24 horas da sua emissão, faça uma reclamação através da opção de menu \"Reclamações\".</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">A empresa registrou outro CPF em minha nota fiscal. Como devo proceder?</h3>\n" +
    "                <p>Você pode solicitar à empresa que faça o cancelamento dessa\n" +
    "                        nota e\n" +
    "                        emita outra corretamente, com o seu CPF. Caso a empresa se negue a corrigir o erro,\n" +
    "                        você\n" +
    "                        poderá encaminhar uma denúncia à Secretaria de Tributação, através do aplicativo ou\n" +
    "                        do site da Campanha Nota Potiguar (appnp.sefaz.rn.gov.br).</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Quais empresas participantes?</h3>\n" +
    "                <p>Todas as empresas estão aptas para o cadastramento do CPF\n" +
    "                        nas notas fiscais. Entretanto, os microempreendedores individuais (MEI) não são obrigados a emitir Nota Fiscal Eletrônica.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Ao consultar minha nota fiscal, foi informado que ela foi emitida em\n" +
    "                        contingência. O que\n" +
    "                        isso significa?</h3>\n" +
    "                <p>Quer dizer que no momento da emissão do documento fiscal\n" +
    "                        ocorreu\n" +
    "                        algum problema técnico que impediu a transmissão da nota fiscal à Tributação Estadual e que a\n" +
    "                        empresa\n" +
    "                        terá o prazo de até 24 horas para transmiti-la.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Posso informar meu CPF depois de emitida a nota fiscal eletrônica?</h3>\n" +
    "                <p>Não. O CPF deve sempre ser informado no momento da compra.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Preciso informar meu nome, endereço e telefone às lojas para participar?</h3>\n" +
    "                <p>Não. Basta informar apenas o seu CPF.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Posso alterar meus dados cadastrados?</h3>\n" +
    "                <p>Sim. A qualquer momento os dados cadastrais podem ser\n" +
    "                        atualizados\n" +
    "                        no app \"Nota Potiguar\", porém nome, CPF e o nome da mãe não poderão ser\n" +
    "                        alterados.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Como posso alterar minha senha?</h3>\n" +
    "                <p>No menu \"Segurança\", você poderá trocar sua senha\n" +
    "                        atual por\n" +
    "                        outra a sua escolha, seguindo as especificações solicitadas pelo aplicativo.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Como posso recuperar minha senha?</h3>\n" +
    "                <p>Ao entrar no aplicativo, o usuário terá a opção \"Redefinir\n" +
    "                        senha\". Nela você informará seu CPF e escolherá por qual meio deseja recuperá-la (SMS ou\n" +
    "                        email). Ao final, receberá instruções para resgatar sua senha através do mecanismo elegido.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Qual a diferença entre Cupom Fiscal e Nota Fiscal?</h3>\n" +
    "                <p>\n" +
    "                        <b>Cupom fiscal</b><br /><br />\n" +
    "                        Documento emitido pela caixa registradora, ele disponibilizava poucas informações e\n" +
    "                        geralmente quase nenhum dado sobre o cliente. Desde o dia 31/12/2017 os Emissores\n" +
    "                        de Cupom\n" +
    "                        Fiscal (ECF’s) dos comércios varejistas do Rio Grande do Norte cessaram o\n" +
    "                        funcionamento. Em\n" +
    "                        caso de uso indevido, o estabelecimento comercial está passível de multa pelo\n" +
    "                        funcionamento\n" +
    "                        fora das regras.<br /><br />\n" +
    "                        <b>Nota fiscal</b><br /><br />\n" +
    "                        Existem dois modelos de nota fiscal: a Nota Fiscal do Consumidor Eletrônica (NFC-e)\n" +
    "                        e a\n" +
    "                        Nota Fiscal Eletrônica (NF-e). A NFC-e é direcionada ao consumidor e vem para\n" +
    "                        substituir o\n" +
    "                        cupom fiscal, já a NF-e atende a todas as outras situações possíveis, desde as\n" +
    "                        operações de\n" +
    "                        compra e venda, até as operações de devolução ou transferência de mercadorias,\n" +
    "                        exportação,\n" +
    "                        etc.\n" +
    "                        A Nota Fiscal é um instrumento mais eficiente para evitar fraudes fiscais, garantir\n" +
    "                        que os\n" +
    "                        direitos do consumidor sejam respeitados, além de ser o documento apropriado para\n" +
    "                        solicitar\n" +
    "                        a garantia dos produtos adquiridos.\n" +
    "                </p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">A empresa pode exigir o cadastro de meu nome, endereço, telefone ou outros\n" +
    "                        dados, para\n" +
    "                        inserir meu CPF na nota fiscal eletrônica?</h3>\n" +
    "                <p>A inclusão do número do CPF no documento fiscal eletrônico\n" +
    "                        não pode\n" +
    "                        ser condicionada a nenhuma espécie de cadastro prévio do consumidor no\n" +
    "                        estabelecimento\n" +
    "                        comercial.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Preciso guardar as notas fiscais impressas?</h3>\n" +
    "                <p>Não é necessário, desde que a nota esteja no aplicativo.\n" +
    "                        Após a sua\n" +
    "                        inclusão, as notas fiscais ficam armazenadas, podendo ser acessadas quando o\n" +
    "                        usuário\n" +
    "                        preferir.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">A empresa pode estipular um valor mínimo de compra para inserir meu CPF na nota\n" +
    "                        fiscal\n" +
    "                        eletrônica?</h3>\n" +
    "                <p>Não. Para qualquer venda, independentemente do valor,\n" +
    "                        deverá ser\n" +
    "                        emitido o documento fiscal.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Minhas notas fiscais serão informadas à Receita Federal?</h3>\n" +
    "                <p>Não há nenhum acordo de transferência de informações entre\n" +
    "                        a\n" +
    "                        Secretaria Estadual de Tributação do RN e a Receita Federal.</p>\n" +
    "                \n" +
    "                \n" +
    "                \n" +
    "                <h3 class=\"titulo-pagina-h3\">Como faço para baixar o aplicativo?</h3>\n" +
    "                <p>\n" +
    "                        I. Na loja Google Play (Android) ou App Store (Apple) pesquise o aplicativo Nota Potiguar (ou baixe através do site np.sefaz.rn.gov.br);<br />\n" +
    "                        II. Clique em \"instalar\";<br />\n" +
    "                        III. Após instalado, clique no ícone do aplicativo no seu celular;<br />\n" +
    "                        IV. Ao abrir o aplicativo clique em \"Faça o seu cadastro\". Informe todos os seus dados corretamente, sem abreviações ou omissões, \n" +
    "                        pois eles serão confirmados de forma automática nas bases de dados da Tributação Estadual.\n" +
    "                        Divergências na grafia dos nomes dos usuários, ou da sua mãe, são as causas mais comuns de rejeição de cadastros;<br />\n" +
    "                        V. Será enviado um e-mail para o endereço informado para confirmação de cadastro;<br />\n" +
    "                        VI. Ao confirmar o cadastro, o aplicativo pode ser aberto novamente para o uso.\n" +
    "                </p>\n" +
    "                \n" +
    "        \n" +
    "            </div>\n" +
    "        </div>\n" +
    "    </div>\n" +
    "</section>");
}]);
