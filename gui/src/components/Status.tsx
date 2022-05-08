import React from 'react';
import styles from './styles/Status.module.css';

type StatusProps = {
  message: string,
  balance: number,
  address: string
};

const Status: React.FC<StatusProps> = ({ message, balance, address }) => {
  return (
    <div className={styles.statusContainer}>
      <div className={styles.status}>
        <h4 className={styles.value}>Playing as: {address}</h4>
      </div>
      <div className={styles.status}>
        <h2 className={styles.value}>{message}</h2>
      </div>
      <div className={styles.balance}>
        <h1 className={styles.value}>Betting on: {balance} u𝕊</h1>
      </div>

    </div>
  );
}

export default Status;