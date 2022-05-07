import React from 'react';
import styles from './styles/Hand.module.css';
import Card from './Card';
import { getConfigFileParsingDiagnostics } from 'typescript';

type HandProps = {
  title: string,
  cards: any[],
  isDealer: boolean,
};

const Hand: React.FC<HandProps> = ({ title, cards, isDealer }) => {
  const getEmptyCard = () => {
    if (cards.length === 0) {
      return (
        <div className={styles.cardContainer}>
          <Card key={0} value={"K"} suit={'♠'} hidden={true}/>
          <Card key={1} value={"K"} suit={'♠'} hidden={true}/>
        </div>
      );
    }
  }

  const getTitle = () => {
    if(isDealer) {
      return (
        <h1 className={styles.title}>{title}</h1>
      );
    } 

    return (
      <h1 className={styles.playerTitle}>{title}</h1>
    );
  }

  return (
    <div className={styles.handContainer}>
      {getTitle()}
      {getEmptyCard()}
      <div className={styles.cardContainer}>
        {cards.map((card: any, index: number) => {
          return (
            <Card key={index} value={card.value} suit={card.suit} hidden={card.hidden} />
          );
        })}
      </div>
    </div>
  );
}

export default Hand;