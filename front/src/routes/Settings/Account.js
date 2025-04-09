import { SettingsLayout } from '../../components/SettingsLayout';
import { AccountDisplayName } from './AccountDisplayName';
import { DeleteAccount } from './DeleteAccount';

export const Account = () => {
    return (
        <SettingsLayout selectedRoute="account">
            <AccountDisplayName />
            <DeleteAccount />
        </SettingsLayout>
    );
};
