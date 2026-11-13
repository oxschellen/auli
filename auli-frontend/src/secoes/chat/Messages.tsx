import { Flex } from "@chakra-ui/react";
import { UserMessage } from "./UserMessage";
import { SystemMessage } from "./SystemMessage";
import { Pensando } from "./Pensando";
import type { Message, SetPrompt } from "./tipos";

interface MessagesProps {
  messages: Message[];
  setPrompt: SetPrompt;
}

export const Messages = ({ messages, setPrompt }: MessagesProps) => {
  return (
    <Flex
      w="100%"
      maxW="760px"
      mx="auto"
      flexDirection="column"
      position="relative"
      bg="bg.app"
      px={1}
      py={1}
    >
      {messages.map((item, index) => {
        const messageText = item.text;
        const key = item.id ?? index;

        if (item.natureza === "pergunta") {
          return <UserMessage key={key} messageText={messageText} setPrompt={setPrompt} />;
        } else if (item.natureza === "espera") {
          return <Pensando key={key} />;
        } else {
          return (
            <SystemMessage
              key={key}
              id={item.id}
              messageText={messageText}
              natureza={item.natureza}
              logId={item.logId}
              fontes={item.fontes}
              tipo={item.tipo}
              duracaoMs={item.duracaoMs}
            />
          );
        }
      })}
    </Flex>
  );
};
