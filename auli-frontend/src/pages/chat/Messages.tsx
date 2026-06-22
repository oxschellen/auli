import { Flex } from "@chakra-ui/react";
import { UserMessage } from "./UserMessage";
import { SystemMessage } from "./SystemMessage";
import type { Message, SetPrompt } from "../../types/chat";

interface MessagesProps {
  messages: Message[];
  setPrompt: SetPrompt;
}

export const Messages = ({ messages, setPrompt }: MessagesProps) => {
  return (
    <Flex
      w="100%"
      flexDirection="column"
      position="relative"
      bg="bg.app"
      px={1}
      py={1}
    >
      {messages.map((item, index) => {
        const messageText = item.text;
        const key = item.id ?? index;

        if (item.from === "user") {
          return <UserMessage key={key} messageText={messageText} setPrompt={setPrompt} />;
        } else {
          return <SystemMessage key={key} messageText={messageText} showButton={item.showButton} />;
        }
      })}
    </Flex>
  );
};
