import assert from 'node:assert'
import test from 'node:test'

import {decodeGithubUrl} from './github.mjs';


test('github URLs', async () => {
    assert.deepEqual(
        decodeGithubUrl('https://github.com/facebook/react'),
        { owner: 'facebook', repository: 'react' });

    assert.deepEqual(
        decodeGithubUrl('https://github.com/facebook/react.git'),
        { owner: 'facebook', repository: 'react' });

    assert.deepEqual(
        decodeGithubUrl('git@github.com:facebook/react.git'),
        { owner: 'facebook', repository: 'react' });

    assert.deepEqual(
        decodeGithubUrl('facebook/react'),
        { owner: 'facebook', repository: 'react' });

    assert.deepEqual(
        decodeGithubUrl('https://github.com/facebook/react/'),
        { owner: 'facebook', repository: 'react' });

    assert.deepEqual(
        decodeGithubUrl('https://github.com/user-name/repo-name'),
        { owner: 'user-name', repository: 'repo-name' });
});
